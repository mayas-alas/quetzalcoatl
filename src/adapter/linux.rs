use super::{
    filesystem::{atomic_write, private_dir},
    process,
    release::PinnedRelease,
};
use crate::{
    config::Config,
    domain::secret::{Secret, SecretKind},
    port::{host::Host, runtime::Runtime},
    report::Capability,
};
use std::{path::PathBuf, time::Duration};
use zeroize::{Zeroize, Zeroizing};
pub struct Linux {
    pub config: Config,
    pub root: PathBuf,
}
fn quadlet(
    name: &str,
    cap: &str,
    instance: &str,
    args: &str,
    requires: &str,
) -> Result<String, String> {
    let words: Vec<_> = args.split_whitespace().collect();
    let image = words
        .iter()
        .position(|v| v.contains("@sha256:"))
        .ok_or("RELEASE_UNPINNED")?;
    let options = words[..image].join(" ");
    let command = words[image + 1..].join(" ");
    Ok(format!("[Unit]\nDescription=GNX {cap} ({instance})\nAfter=network-online.target {requires}\nWants=network-online.target\nRequires={requires}\nBindsTo={requires}\nPartOf={requires}\nStartLimitIntervalSec=300\nStartLimitBurst=5\n[Container]\nImage={}\nContainerName={name}\nPodmanArgs=--pull=never --log-driver=none {options}\nExec={command}\n[Service]\nRestart=on-failure\nRestartSec=5\nTimeoutStartSec=300\nTimeoutStopSec=120\nStandardOutput=null\nStandardError=null\n[Install]\nWantedBy=multi-user.target\n", words[image]))
}
impl Linux {
    pub fn new(config: Config) -> Self {
        let root = PathBuf::from(format!("/var/lib/gnx/{}", config.instance));
        Self { config, root }
    }
    pub fn name(&self, cap: &str) -> String {
        format!("gnx-{}-{cap}", self.config.instance)
    }
    pub fn path(&self, s: &str) -> String {
        self.root.join(s).to_string_lossy().into_owned()
    }
    fn network(&self) -> String {
        format!("gnx-{}", self.config.instance)
    }
    fn network_ip(&self, offset: u32) -> String {
        let ip = self
            .config
            .network
            .subnet
            .split('/')
            .next()
            .unwrap()
            .parse::<std::net::Ipv4Addr>()
            .unwrap();
        std::net::Ipv4Addr::from(u32::from(ip) + offset).to_string()
    }
    fn compute_ip(&self) -> String {
        self.network_ip(2)
    }
    fn access_ip(&self) -> String {
        self.network_ip(3)
    }
    fn exec(
        &self,
        cap: &str,
        args: &[&str],
        input: Option<&[u8]>,
    ) -> Result<Zeroizing<Vec<u8>>, String> {
        let name = self.name(cap);
        let mut a = vec!["exec"];
        if input.is_some() {
            a.push("-i")
        }
        a.push(&name);
        a.extend_from_slice(args);
        process::checked("podman", &a, input, 30)
    }
    fn image(&self, cap: &str, image: &str) -> Result<(), String> {
        let b = process::checked(
            "podman",
            &["inspect", "--format", "{{.ImageName}}", &self.name(cap)],
            None,
            10,
        )?;
        if String::from_utf8_lossy(&b).trim() != image {
            return Err("ARTIFACT_MISMATCH".into());
        }
        Ok(())
    }
    fn ensure_images(&self, r: &PinnedRelease) -> Result<(), String> {
        for image in [&r.compute, &r.access, &r.dns, &r.control] {
            let o = process::run(
                "podman",
                &["image", "exists", image],
                None,
                Duration::from_secs(10),
                1024,
            )?;
            if o.code != 0 {
                process::checked("podman", &["pull", "--quiet", image], None, 300)
                    .map_err(|_| "IMAGE_PULL_FAILED")?;
            }
        }
        Ok(())
    }
    fn ensure_network(&self) -> Result<(), String> {
        let name = self.network();
        let o = process::run(
            "podman",
            &["network", "inspect", &name],
            None,
            Duration::from_secs(10),
            65536,
        )?;
        if o.code == 0 {
            let v: serde_json::Value =
                serde_json::from_slice(&o.stdout).map_err(|_| "NETWORK_INSPECT_FAILED")?;
            if v[0]["labels"]["io.gnx.instance"] != self.config.instance
                || v[0]["subnets"][0]["subnet"] != self.config.network.subnet
            {
                return Err("NETWORK_CONFLICT".into());
            }
            return Ok(());
        }
        process::checked(
            "podman",
            &[
                "network",
                "create",
                "--label",
                &format!("io.gnx.instance={}", self.config.instance),
                "--subnet",
                &self.config.network.subnet,
                &name,
            ],
            None,
            30,
        )
        .map_err(|_| "NETWORK_CREATE_FAILED")?;
        Ok(())
    }
    fn unit(&self, cap: &str, args: &str, requires: &str) -> Result<(), String> {
        let name = self.name(cap);
        let path = PathBuf::from(format!("/etc/containers/systemd/{name}.container"));
        std::fs::create_dir_all("/etc/containers/systemd")
            .map_err(|_| "QUADLET_DIRECTORY_FAILED")?;
        let content = quadlet(&name, cap, &self.config.instance, args, requires)?;
        let content = if cap == "access" {
            content.replacen(
                "[Unit]\n",
                &format!(
                    "[Unit]\nWants={}.service {}.service\n",
                    self.name("dns"),
                    self.name("control")
                ),
                1,
            )
        } else {
            content
        };
        // Migrate only this instance's generated development unit; retain its original file.
        let legacy = PathBuf::from(format!("/etc/systemd/system/{name}.service"));
        if legacy.exists() {
            let old = std::fs::read_to_string(&legacy).map_err(|_| "UNIT_READ_FAILED")?;
            if !old.contains(&format!("Description=GNX {cap} ({})", self.config.instance)) {
                return Err("UNIT_OWNERSHIP_CONFLICT".into());
            }
            std::fs::rename(&legacy, legacy.with_extension("service.pre-quadlet"))
                .map_err(|_| "UNIT_MIGRATION_FAILED")?;
        }
        let changed = std::fs::read(&path).ok().as_deref() != Some(content.as_bytes());
        if changed {
            atomic_write(&path, content.as_bytes(), 0o644)?;
            process::checked("systemctl", &["daemon-reload"], None, 30)?;
        }
        // Quadlet's generator implements [Install]; generated units cannot be enabled directly.
        if changed {
            process::checked(
                "systemctl",
                &["restart", &format!("{name}.service")],
                None,
                150,
            )?;
        } else if !self.active(cap) {
            process::checked(
                "systemctl",
                &["start", &format!("{name}.service")],
                None,
                150,
            )?;
        }
        Ok(())
    }
    fn active(&self, cap: &str) -> bool {
        process::run(
            "systemctl",
            &[
                "is-active",
                "--quiet",
                &format!("{}.service", self.name(cap)),
            ],
            None,
            Duration::from_secs(10),
            1024,
        )
        .is_ok_and(|o| o.code == 0)
    }
    fn compute_start(&self, r: &PinnedRelease, secret: Option<&Secret>) -> Result<(), String> {
        for dir in ["compute", "compute/vz", "compute/cluster"] {
            private_dir(&self.root.join(dir))?;
        }
        let password = self.root.join("compute/password");
        if !password.exists() {
            let s = secret
                .filter(|s| s.kind == SecretKind::ComputePassword)
                .ok_or("COMPUTE_PASSWORD_REQUIRED")?;
            atomic_write(&password, &s.value, 0o600)?;
        }
        let wrapper = include_str!("../../runtime/compute/entrypoint.sh").replace('\r', "");
        let wrapper_path = self.root.join("compute/entrypoint.sh");
        if std::fs::read(&wrapper_path).ok().as_deref() != Some(wrapper.as_bytes()) {
            atomic_write(&wrapper_path, wrapper.as_bytes(), 0o700)?;
        }
        let args=format!("--network {} --ip {} --hostname {} --privileged --systemd=always --security-opt label=disable --volume {}:/var/lib/vz --volume {}:/var/lib/pve-cluster --volume {}:/run/gnx/password:ro --volume {}:/gnx-entrypoint.sh:ro --volume /dev/null:/var/log/pveproxy/access.log --entrypoint /bin/bash {} /gnx-entrypoint.sh /sbin/init --log-target=console --log-level=notice",self.network(),self.compute_ip(),self.name("compute"),self.path("compute/vz"),self.path("compute/cluster"),self.path("compute/password"),self.path("compute/entrypoint.sh"),r.compute);
        self.unit("compute", &args, "")?;
        for _ in 0..90 {
            if self
                .exec("compute", &["test", "-s", "/etc/pve/pve-root-ca.pem"], None)
                .is_ok()
            {
                let ca = self.exec("compute", &["cat", "/etc/pve/pve-root-ca.pem"], None)?;
                let path = self.root.join("compute/root-ca.pem");
                if std::fs::read(&path).ok().as_deref() != Some(ca.as_slice()) {
                    atomic_write(&path, &ca, 0o644)?;
                }
                if self.compute_health().is_ok() {
                    return Ok(());
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
        Err("COMPUTE_START_TIMEOUT".into())
    }

    fn prepare_secrets(&self, secret: Option<&Secret>) -> Result<(), String> {
        let password = self.root.join("compute/password");
        if !password.exists() {
            let supplied = secret
                .filter(|value| value.kind == SecretKind::ComputePassword)
                .ok_or("COMPUTE_PASSWORD_REQUIRED")?;
            private_dir(&self.root)?;
            private_dir(&self.root.join("compute"))?;
            atomic_write(&password, &supplied.value, 0o600)?;
        }
        if !self.root.join("access/tailscaled.state").exists()
            && !secret.is_some_and(|value| value.kind == SecretKind::AccessEnrollment)
        {
            return Err("ACCESS_ENROLLMENT_REQUIRED".into());
        }
        Ok(())
    }
    fn curl(
        &self,
        url: &str,
        config: &[u8],
        resolve: Option<&str>,
        ca: Option<&str>,
    ) -> Result<Zeroizing<Vec<u8>>, String> {
        let mut a = vec![
            "--silent",
            "--show-error",
            "--fail",
            "--max-time",
            "15",
            "--noproxy",
            "*",
            "--config",
            "-",
        ];
        if let Some(v) = resolve {
            a.extend(["--resolve", v])
        }
        if let Some(v) = ca {
            a.extend(["--cacert", v])
        }
        a.push(url);
        process::checked("curl", &a, Some(config), 20)
    }
    pub fn compute_health(&self) -> Result<(), String> {
        let r = PinnedRelease::embedded()?;
        if !self.active("compute") {
            return Err("SERVICE_NOT_ACTIVE".into());
        }
        self.image("compute", &r.compute)?;
        let password = Zeroizing::new(
            std::fs::read(self.root.join("compute/password"))
                .map_err(|_| "COMPUTE_PASSWORD_REQUIRED")?,
        );
        let encoded =
            Zeroizing::new(url::form_urlencoded::byte_serialize(&password).collect::<String>());
        let config = Zeroizing::new(format!(
            "data = \"username=root%40pam&password={}\"\n",
            *encoded
        ));
        let resolve = format!("{}:8006:{}", self.name("compute"), self.compute_ip());
        let ca = self.path("compute/root-ca.pem");
        let origin = format!("https://{}:8006", self.name("compute"));
        let reply = self
            .curl(
                &format!("{origin}/api2/json/access/ticket"),
                config.as_bytes(),
                Some(&resolve),
                Some(&ca),
            )
            .map_err(|_| "COMPUTE_AUTH_OR_TLS_FAILED")?;
        let mut v: serde_json::Value =
            serde_json::from_slice(&reply).map_err(|_| "COMPUTE_AUTH_FAILED")?;
        let ticket = Zeroizing::new(
            v["data"]["ticket"]
                .as_str()
                .ok_or("COMPUTE_AUTH_FAILED")?
                .to_string(),
        );
        fn wipe(v: &mut serde_json::Value) {
            match v {
                serde_json::Value::String(s) => s.zeroize(),
                serde_json::Value::Array(a) => a.iter_mut().for_each(wipe),
                serde_json::Value::Object(o) => o.values_mut().for_each(wipe),
                _ => (),
            }
        }
        wipe(&mut v);
        if ticket.contains(['"', '\r', '\n', '\\']) {
            return Err("COMPUTE_AUTH_FAILED".into());
        }
        let cookie = Zeroizing::new(format!("header = \"Cookie: PVEAuthCookie={}\"\n", *ticket));
        let reply = self
            .curl(
                &format!("{origin}/api2/json/nodes"),
                cookie.as_bytes(),
                Some(&resolve),
                Some(&ca),
            )
            .map_err(|_| "COMPUTE_AUTH_FAILED")?;
        let v: serde_json::Value =
            serde_json::from_slice(&reply).map_err(|_| "COMPUTE_IDENTITY_FAILED")?;
        if !v["data"].as_array().is_some_and(|nodes| {
            nodes
                .iter()
                .any(|n| n["node"] == self.name("compute") && n["status"] == "online")
        }) {
            return Err("COMPUTE_IDENTITY_FAILED".into());
        }
        let reply = self
            .curl(
                &format!(
                    "{origin}/api2/json/nodes/{}/storage/local/status",
                    self.name("compute")
                ),
                cookie.as_bytes(),
                Some(&resolve),
                Some(&ca),
            )
            .map_err(|_| "COMPUTE_STORAGE_FAILED")?;
        let v: serde_json::Value =
            serde_json::from_slice(&reply).map_err(|_| "COMPUTE_STORAGE_FAILED")?;
        if v["data"]["active"] != 1 {
            return Err("COMPUTE_STORAGE_FAILED".into());
        }
        Ok(())
    }
    fn identity(&self) -> Result<String, String> {
        let b = self
            .exec(
                "access",
                &[
                    "tailscale",
                    "--socket=/run/tailscale/tailscaled.sock",
                    "status",
                    "--json",
                ],
                None,
            )
            .map_err(|_| "ACCESS_ENROLLMENT_REQUIRED")?;
        let v: serde_json::Value =
            serde_json::from_slice(&b).map_err(|_| "ACCESS_ENROLLMENT_REQUIRED")?;
        if v["BackendState"] != "Running" {
            return Err("ACCESS_ENROLLMENT_REQUIRED".into());
        }
        let ip = v["TailscaleIPs"]
            .as_array()
            .and_then(|a| {
                a.iter()
                    .filter_map(|i| i.as_str())
                    .find(|i| i.parse::<std::net::Ipv4Addr>().is_ok())
            })
            .ok_or("ACCESS_IDENTITY_MISSING")?;
        if self
            .config
            .network
            .identity_ip
            .is_some_and(|x| x.to_string() != ip)
        {
            return Err("ACCESS_IDENTITY_MISMATCH".into());
        }
        Ok(ip.into())
    }
    fn access_start(&self, r: &PinnedRelease, secret: Option<&Secret>) -> Result<String, String> {
        private_dir(&self.root.join("access"))?;
        // Reserve a stable address for Access so Podman's allocator cannot take
        // Compute's fixed .2 address when Access starts first after a reboot.
        let args=format!("--network {} --ip {} --cap-add NET_ADMIN --cap-add NET_RAW --device /dev/net/tun --volume {}:/var/lib/tailscale --entrypoint tailscaled {} --state=/var/lib/tailscale/tailscaled.state --socket=/run/tailscale/tailscaled.sock --tun=tailscale0",self.network(),self.access_ip(),self.path("access"),r.access);
        self.unit("access", &args, "")?;
        for _ in 0..15 {
            if self
                .exec(
                    "access",
                    &["test", "-S", "/run/tailscale/tailscaled.sock"],
                    None,
                )
                .is_ok()
            {
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        if let Ok(ip) = self.identity() {
            return Ok(ip);
        }
        let secret = secret
            .filter(|s| s.kind == SecretKind::AccessEnrollment)
            .ok_or("ACCESS_ENROLLMENT_REQUIRED")?;
        // An anonymous stdin stream becomes a root-only tmpfs file for the upstream file: interface.
        self.exec(
            "access",
            &["sh", "-c", "umask 077; cat > /run/gnx-enrollment"],
            Some(&secret.value),
        )?;
        let enrollment = self.exec(
            "access",
            &[
                "tailscale",
                "--socket=/run/tailscale/tailscaled.sock",
                "up",
                "--auth-key=file:/run/gnx-enrollment",
                "--accept-dns=false",
                "--hostname",
                &self.name("node"),
            ],
            None,
        );
        let _ = self.exec("access", &["rm", "-f", "/run/gnx-enrollment"], None);
        enrollment.map_err(|_| "ACCESS_ENROLLMENT_FAILED")?;
        self.identity()
    }
    fn control_start(&self, r: &PinnedRelease, ip: &str) -> Result<(), String> {
        for dir in [
            "control",
            "control/data",
            "control/config",
            "control/app",
            "dns",
        ] {
            private_dir(&self.root.join(dir))?;
        }
        for (name, bytes) in [
            (
                "index.html",
                include_bytes!("../../runtime/control/app/index.html").as_slice(),
            ),
            (
                "style.css",
                include_bytes!("../../runtime/control/app/style.css").as_slice(),
            ),
            (
                "app.js",
                include_bytes!("../../runtime/control/app/app.js").as_slice(),
            ),
        ] {
            let path = self.root.join("control/app").join(name);
            if std::fs::read(&path).ok().as_deref() != Some(bytes) {
                atomic_write(&path, bytes, 0o644)?;
            }
        }
        let core = super::coredns::render_at(&self.config, ip)?;
        let caddy =
            super::caddy::render(&self.config, ip, &self.name("compute"), &self.compute_ip());
        let zone = super::coredns::zone(&self.config, ip);
        let dns_changed = std::fs::read(self.root.join("dns/gnx.zone"))
            .ok()
            .as_deref()
            != Some(zone.as_bytes())
            || std::fs::read(self.root.join("dns/Corefile"))
                .ok()
                .as_deref()
                != Some(core.as_bytes());
        let control_changed = std::fs::read(self.root.join("control/Caddyfile"))
            .ok()
            .as_deref()
            != Some(caddy.as_bytes());
        if dns_changed {
            atomic_write(&self.root.join("dns/Corefile"), core.as_bytes(), 0o644)?;
        }
        if control_changed {
            atomic_write(
                &self.root.join("control/Caddyfile"),
                caddy.as_bytes(),
                0o600,
            )?;
        }
        let requires = format!("{}.service", self.name("access"));
        if dns_changed {
            atomic_write(&self.root.join("dns/gnx.zone"), zone.as_bytes(), 0o644)?;
        }
        self.unit("dns",&format!("--network container:{} --volume {}:/Corefile:ro --volume {}:/gnx.zone:ro {} -conf /Corefile",self.name("access"),self.path("dns/Corefile"),self.path("dns/gnx.zone"),r.dns),&requires)?;
        self.unit("control",&format!("--network container:{} --volume {}:/etc/caddy/Caddyfile:ro --volume {}:/data --volume {}:/config --volume {}:/gnx-compute-ca.pem:ro --volume {}:/gnx-app:ro {} caddy run --config /etc/caddy/Caddyfile --adapter caddyfile",self.name("access"),self.path("control/Caddyfile"),self.path("control/data"),self.path("control/config"),self.path("compute/root-ca.pem"),self.path("control/app"),r.control),&requires)?;
        if dns_changed {
            process::checked(
                "systemctl",
                &["restart", &format!("{}.service", self.name("dns"))],
                None,
                30,
            )?;
        }
        if control_changed {
            process::checked(
                "systemctl",
                &["restart", &format!("{}.service", self.name("control"))],
                None,
                30,
            )?;
        }
        for _ in 0..30 {
            if self.control_health().is_ok() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        Err("CONTROL_START_TIMEOUT".into())
    }
    fn access_health(&self) -> Result<(), String> {
        let ip = self.identity()?;
        let r = PinnedRelease::embedded()?;
        self.image("access", &r.access)?;
        self.image("dns", &r.dns)?;
        super::coredns::probe(&ip, &self.config)
    }
    fn curl_private(
        &self,
        url: &str,
        config: &[u8],
        resolve: &str,
        ca: &str,
    ) -> Result<Zeroizing<Vec<u8>>, String> {
        let b = process::checked(
            "podman",
            &[
                "inspect",
                "--format",
                "{{.State.Pid}}",
                &self.name("access"),
            ],
            None,
            10,
        )?;
        let pid = String::from_utf8_lossy(&b)
            .trim()
            .parse::<u32>()
            .map_err(|_| "ACCESS_NAMESPACE_MISSING")?
            .to_string();
        if pid == "0" {
            return Err("ACCESS_NAMESPACE_MISSING".into());
        }
        process::checked(
            "nsenter",
            &[
                "--target",
                &pid,
                "--net",
                "--",
                "curl",
                "--silent",
                "--fail",
                "--max-time",
                "15",
                "--noproxy",
                "*",
                "--config",
                "-",
                "--resolve",
                resolve,
                "--cacert",
                ca,
                url,
            ],
            Some(config),
            20,
        )
    }
    fn control_health(&self) -> Result<(), String> {
        let ip = self.identity()?;
        let r = PinnedRelease::embedded()?;
        self.image("control", &r.control)?;
        let ca = self.path("control/data/caddy/pki/authorities/local/root.crt");
        let resolve = format!("compute.gnx:443:{ip}");
        let b = self
            .curl_private("https://compute.gnx/", b"", &resolve, &ca)
            .map_err(|_| "CONTROL_TLS_OR_UPSTREAM_FAILED")?;
        if !String::from_utf8_lossy(&b).contains("Proxmox") {
            return Err("CONTROL_UPSTREAM_IDENTITY_FAILED".into());
        }
        Ok(())
    }
}
impl Host for Linux {
    fn prerequisites(&self) -> Result<(), String> {
        if !cfg!(target_os = "linux") {
            return Err("LINUX_REQUIRED".into());
        }
        #[cfg(unix)]
        if unsafe { libc::geteuid() } != 0 {
            return Err("ROOT_REQUIRED".into());
        }
        for (path, code) in [
            (
                "/usr/lib/systemd/system-generators/podman-system-generator",
                "QUADLET_REQUIRED",
            ),
            ("/run/systemd/system", "SYSTEMD_REQUIRED"),
            ("/sys/fs/cgroup/cgroup.controllers", "CGROUP_V2_REQUIRED"),
            ("/dev/kvm", "KVM_REQUIRED"),
            ("/dev/fuse", "FUSE_REQUIRED"),
            ("/dev/net/tun", "TUN_REQUIRED"),
        ] {
            if !std::path::Path::new(path).exists() {
                return Err(code.into());
            }
        }
        PinnedRelease::embedded()?;
        if !super::podman::available() {
            return Err("PODMAN_REQUIRED".into());
        }
        for p in ["curl", "openssl", "nsenter", "dig"] {
            process::checked(p, &["--version"], None, 10)
                .or_else(|_| process::checked(p, &["version"], None, 10))
                .map_err(|_| "TOOLS_REQUIRED")?;
        }
        if fs2::available_space("/var/lib").map_err(|_| "STORAGE_OBSERVATION_FAILED")?
            < 32 * 1024 * 1024 * 1024
        {
            return Err("STORAGE_REQUIRED".into());
        }
        Ok(())
    }
}
impl Runtime for Linux {
    fn revision(&self, c: &Config) -> String {
        use sha2::{Digest, Sha256};
        let mut digest = Sha256::new();
        digest.update(b"GNX-RUNTIME-REVISION-V1\0");
        digest.update(c.revision().as_bytes());
        digest.update(crate::adapter::release::DEFINITION.as_bytes());
        hex::encode(digest.finalize())
    }
    fn observe(&self) -> Vec<Capability> {
        [
            ("access", self.access_health()),
            ("control", self.control_health()),
            ("compute", self.compute_health()),
        ]
        .into_iter()
        .map(|(name, result)| Capability {
            name: name.into(),
            healthy: result.is_ok(),
            code: result.err().unwrap_or("OK".into()),
        })
        .collect()
    }
    fn reconcile(&self, c: &Config) -> Result<(), String> {
        self.reconcile_secret(c, None)
    }
    fn reconcile_secret(&self, _: &Config, secret: Option<&Secret>) -> Result<(), String> {
        let release = PinnedRelease::embedded()?;
        // Complete the typed secret handshake before downloading images or
        // creating networks, units and persistent service storage.
        self.prepare_secrets(secret)?;
        self.ensure_images(&release)?;
        self.ensure_network()?;
        self.compute_start(&release, secret)?;
        let ip = self.access_start(&release, secret)?;
        self.control_start(&release, &ip)
    }
    fn restore(&self, previous: Option<&Config>) -> Result<(), String> {
        if let Some(c) = previous {
            let old = Self::new(c.clone());
            old.reconcile(c)?;
        }
        Ok(())
    }
    fn public_root(&self) -> Option<String> {
        std::fs::read_to_string(
            self.root
                .join("control/data/caddy/pki/authorities/local/root.crt"),
        )
        .ok()
        .filter(|s| s.starts_with("-----BEGIN CERTIFICATE-----") && s.len() < 16384)
    }
    fn access_ip(&self) -> Option<String> {
        self.identity().ok()
    }
    fn optional_routes(&self) -> Vec<Capability> {
        self.config
            .routes
            .iter()
            .map(|r| {
                let result = process::run(
                    "curl",
                    &[
                        "--silent",
                        "--fail",
                        "--max-time",
                        "5",
                        "--noproxy",
                        "*",
                        "--output",
                        "/dev/null",
                        &r.upstream,
                    ],
                    None,
                    Duration::from_secs(7),
                    1024,
                );
                let healthy = result.is_ok_and(|o| o.code == 0);
                Capability {
                    name: r.hostname.clone(),
                    healthy,
                    code: if healthy {
                        "OK"
                    } else {
                        "UPSTREAM_UNREACHABLE"
                    }
                    .into(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod local_integration {
    use super::*;

    #[test]
    fn reserves_distinct_access_and_compute_addresses() {
        let config = Config::parse(
            "schema=1\ninstance='poc031'\nnode='compute'\n[network]\nsubnet='10.93.0.0/24'\n",
        )
        .unwrap();
        let linux = Linux::new(config);
        assert_eq!(linux.compute_ip(), "10.93.0.2");
        assert_eq!(linux.access_ip(), "10.93.0.3");
    }

    #[test]
    #[ignore = "Creates local DNS/TLS units in the explicitly named poc031 lab namespace; run as Linux root only"]
    fn dns_tls_loopback() {
        let config = Config::parse(
            "schema=1\ninstance='poc031'\nnode='compute'\n[network]\nsubnet='10.93.0.0/24'\n",
        )
        .unwrap();
        let linux = Linux::new(config);
        let release = PinnedRelease::embedded().unwrap();
        // The un-enrolled namespace is isolated; localhost is never a product entry address.
        let _ = linux.control_start(&release, "127.0.0.1");
        super::super::coredns::probe("127.0.0.1", &linux.config).unwrap();
        let ca = linux.path("control/data/caddy/pki/authorities/local/root.crt");
        let response = linux
            .curl_private(
                "https://compute.gnx/",
                b"",
                "compute.gnx:443:127.0.0.1",
                &ca,
            )
            .unwrap();
        assert!(String::from_utf8_lossy(&response).contains("Proxmox"));
    }
}
