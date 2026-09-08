use crate::{Result, config::Config, plan, release, report::Failure};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream, UdpSocket},
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        io::AsRawFd,
    },
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub fn command(args: &[&str], input: Option<&[u8]>, seconds: u64) -> Result<Vec<u8>> {
    let stdout = tempfile::tempfile()?;
    let stderr = tempfile::tempfile()?;
    let mut child = Command::new(args[0])
        .args(&args[1..])
        .env_remove("TS_AUTHKEY")
        .env_remove("PASSWORD")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?)
        .spawn()
        .map_err(|_| {
            Failure::action(
                "COMMAND_MISSING",
                format!(
                    "Cannot start {}; install the required runtime tools",
                    args[0]
                ),
            )
        })?;
    if let Some(data) = input {
        if let Err(error) = child.stdin.take().expect("piped stdin").write_all(data) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
    }
    let start = Instant::now();
    let exit = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(seconds) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Failure::new(
                "COMMAND_TIMEOUT",
                format!("{} exceeded {seconds}s", args[0]),
            ));
        }
        thread::sleep(Duration::from_millis(50));
    };
    if !exit.success() {
        // Raw stderr, argv and API replies can contain credentials. Keep failures typed.
        return Err(Failure::new(
            "COMMAND_FAILED",
            format!(
                "{} exited with {}; inspect its local diagnostics",
                args[0],
                exit.code().unwrap_or(-1)
            ),
        ));
    }
    use std::io::{Seek, SeekFrom};
    let mut stdout = stdout;
    stdout.seek(SeekFrom::Start(0))?;
    let mut data = Vec::new();
    stdout.take(2_000_001).read_to_end(&mut data)?;
    if data.len() > 2_000_000 {
        return Err(Failure::new(
            "OUTPUT_LIMIT",
            "Command output exceeds the bounded response size",
        ));
    }
    Ok(data)
}

fn root() -> Result<()> {
    if unsafe { libc::geteuid() } != 0 {
        return Err(Failure::action(
            "ROOT_REQUIRED",
            "Run the Linux runtime as the authorized administrator",
        ));
    }
    Ok(())
}

fn check_ancestors(path: &Path) -> Result<()> {
    for parent in path.ancestors() {
        match fs::symlink_metadata(parent) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(Failure::new(
                    "SYMLINK_PATH",
                    format!("Refusing symlink: {}", parent.display()),
                ));
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        }
    }
    Ok(())
}

fn private_dir(path: &Path) -> Result<()> {
    check_ancestors(path)?;
    if path.exists() {
        let m = fs::metadata(path)?;
        if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } {
            return Err(Failure::new(
                "STATE_OWNER",
                "State directory is not owned by the runtime administrator",
            ));
        }
    }
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn atomic(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    check_ancestors(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| Failure::new("PATH", "Missing parent"))?;
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.as_file()
        .set_permissions(fs::Permissions::from_mode(mode))?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| Failure::from(e.error))?;
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

fn lock(config: &Config) -> Result<fs::File> {
    let state = Path::new(&config.state_dir);
    check_ancestors(state)?;
    if state.exists()
        && fs::read_dir(state)?.next().is_some()
        && fs::read_to_string(state.join(".owner")).ok().as_deref() != Some(&config.instance)
    {
        return Err(Failure::new(
            "STATE_OWNERSHIP",
            "Preserving an existing state directory; select a new instance and state_dir",
        ));
    }
    private_dir(Path::new(&config.state_dir))?;
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(Path::new(&config.state_dir).join("apply.lock"))?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(Failure::action(
            "NODE_BUSY",
            "Another apply owns this node; retry after it completes",
        ));
    }
    atomic(&state.join(".owner"), config.instance.as_bytes(), 0o600)?;
    Ok(file)
}

fn checkpoint(config: &Config, phase: &str) -> Result<()> {
    atomic(
        &Path::new(&config.state_dir).join("apply.json"),
        &serde_json::to_vec(&json!({"revision":config.revision(),"phase":phase})).unwrap(),
        0o600,
    )
}

fn systemctl(action: &str, config: &Config, capability: &str) -> Result<()> {
    command(
        &[
            "systemctl",
            action,
            &format!("{}.service", config.unit(capability)),
        ],
        None,
        920,
    )
    .map(|_| ())
}

pub fn doctor(config: &Config) -> Result<Value> {
    let mut checks = Vec::new();
    checks.push(json!({"check":"systemd","ok":fs::read_to_string("/proc/1/comm").unwrap_or_default().trim()=="systemd"}));
    checks.push(
        json!({"check":"cgroup_v2","ok":Path::new("/sys/fs/cgroup/cgroup.controllers").exists()}),
    );
    for path in ["/dev/kvm", "/dev/net/tun"] {
        let ok = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map(|f| path != "/dev/kvm" || unsafe { libc::ioctl(f.as_raw_fd(), 0xAE00) } == 12)
            .unwrap_or(false);
        checks.push(json!({"check":path,"ok":ok}));
    }
    for program in ["podman", "systemctl", "nsenter"] {
        checks.push(json!({"check":program,"ok":command(&[program,"--version"],None,10).is_ok()}));
    }
    checks.push(json!({"check":"quadlet","ok":Path::new("/usr/lib/systemd/system-generators/podman-system-generator").exists()}));
    if checks.iter().any(|c| c["ok"] != true) {
        return Err(Failure::action(
            "PREFLIGHT",
            format!(
                "Missing capabilities: {}",
                checks
                    .iter()
                    .filter(|c| c["ok"] != true)
                    .map(|c| c["check"].as_str().unwrap_or("unknown"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }
    Ok(
        json!({"revision":config.revision(),"checks":checks,"note":"Remote DNS, TLS and reboot acceptance are separate gates"}),
    )
}

fn private_password(config: &Config) -> Result<String> {
    let path = Path::new(&config.state_dir).join("compute/private/password");
    check_ancestors(&path)?;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    let meta = file.metadata()?;
    if meta.mode() & 0o077 != 0 || meta.uid() != unsafe { libc::geteuid() } || !meta.is_file() {
        return Err(Failure::new(
            "SECRET_PERMISSIONS",
            "Compute credential must be a private administrator-owned file",
        ));
    }
    let mut password = String::new();
    file.take(1025).read_to_string(&mut password)?;
    if password.trim().len() < 16 || password.len() > 1024 {
        return Err(Failure::new("SECRET_FORMAT", "Invalid Compute credential"));
    }
    Ok(password.trim().into())
}

pub fn access_ip(config: &Config) -> Result<Ipv4Addr> {
    let output = command(
        &[
            "podman",
            "exec",
            &config.unit("access"),
            "tailscale",
            "status",
            "--json",
        ],
        None,
        15,
    )?;
    let status: Value = serde_json::from_slice(&output)
        .map_err(|_| Failure::new("ACCESS_STATUS", "Invalid Access status response"))?;
    if status["BackendState"] != "Running" {
        return Err(Failure::action(
            "ACCESS_ENROLLMENT",
            "Run gnx access enroll with a protected credential on stdin, then configure restricted DNS",
        ));
    }
    let ip = status["TailscaleIPs"]
        .as_array()
        .and_then(|a| a.iter().find_map(|v| v.as_str()?.parse::<Ipv4Addr>().ok()))
        .ok_or_else(|| Failure::action("ACCESS_IP", "Access has no private IPv4 address"))?;
    if config
        .network
        .identity_ip
        .is_some_and(|expected| expected != ip)
    {
        return Err(Failure::action(
            "ACCESS_IP_CHANGED",
            "Observed Access identity differs from configuration; reconcile DNS and intent",
        ));
    }
    Ok(ip)
}

pub fn enroll(config: &Config, key: &[u8]) -> Result<Value> {
    root()?;
    if key.len() > 4096 || key.is_empty() {
        return Err(Failure::new(
            "ENROLL_INPUT",
            "Provide a single Access enrollment credential on stdin",
        ));
    }
    const ENROLL: &str = "set -eu; umask 077; key=$(mktemp /run/tailscale/enroll.XXXXXX); trap 'rm -f \"$key\"' EXIT; cat > \"$key\"; tailscale up --auth-key=file:$key --accept-dns=false --accept-routes=false --hostname=\"$1\" > /dev/null";
    command(
        &[
            "podman",
            "exec",
            "-i",
            &config.unit("access"),
            "sh",
            "-c",
            ENROLL,
            "enroll",
            &config.instance,
        ],
        Some(key),
        120,
    )?;
    Ok(
        json!({"ip":access_ip(config)?,"next":"Configure restricted nameserver gnx to this IP; run gnx apply"}),
    )
}

fn export_ca(config: &Config) -> Result<()> {
    let ca = command(
        &[
            "podman",
            "exec",
            &config.unit("compute"),
            "cat",
            "/etc/pve/pve-root-ca.pem",
        ],
        None,
        15,
    )?;
    reqwest::Certificate::from_pem(&ca)
        .map_err(|_| Failure::new("COMPUTE_CA", "Compute did not provide a valid CA"))?;
    let path = Path::new(&config.state_dir).join("compute/public/upstream-ca.crt");
    if fs::read(&path).ok().as_deref() != Some(&ca) {
        atomic(&path, &ca, 0o644)?;
    }
    Ok(())
}

pub fn compute_status(config: &Config) -> Result<Value> {
    systemctl("is-active", config, "compute")?;
    let ca = fs::read(Path::new(&config.state_dir).join("compute/public/upstream-ca.crt"))?;
    let ca = reqwest::Certificate::from_pem(&ca)
        .map_err(|_| Failure::new("COMPUTE_CA", "Invalid stored Compute CA"))?;
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .tls_built_in_root_certs(false)
        .add_root_certificate(ca)
        .resolve(
            &config.node,
            SocketAddr::from((config.network.compute_ip, 8006)),
        )
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| Failure::new("HTTP_CLIENT", "Cannot initialize TLS client"))?;
    let endpoint = format!("https://{}:8006/api2/json", config.node);
    let password = private_password(config)?;
    let login: Value = client
        .post(format!("{endpoint}/access/ticket"))
        .form(&[("username", "root@pam"), ("password", password.as_str())])
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|_| {
            Failure::new(
                "COMPUTE_AUTH_TLS",
                "Authenticated Compute HTTPS failed; check trust, name, credentials and reachability",
            )
        })?;
    if login["data"]["username"] != "root@pam" {
        return Err(Failure::new(
            "COMPUTE_IDENTITY",
            "Unexpected Compute identity",
        ));
    }
    let ticket = login["data"]["ticket"]
        .as_str()
        .ok_or_else(|| Failure::new("COMPUTE_AUTH", "Missing authentication ticket"))?;
    let mut responses = Vec::new();
    for path in [
        format!("nodes/{}/status", config.node),
        format!("nodes/{}/storage", config.node),
    ] {
        let value: Value = client
            .get(format!("{endpoint}/{path}"))
            .header("Cookie", format!("PVEAuthCookie={ticket}"))
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.json())
            .map_err(|_| {
                Failure::new(
                    "COMPUTE_HEALTH",
                    "Authenticated Compute node/storage query failed",
                )
            })?;
        responses.push(value);
    }
    if responses[0]["data"]["uptime"].as_u64().unwrap_or(0) == 0
        || !responses[1]["data"]
            .as_array()
            .is_some_and(|s| s.iter().any(|v| v["active"] == 1))
    {
        return Err(Failure::new(
            "COMPUTE_STORAGE",
            "Compute node or storage is not ready",
        ));
    }
    Ok(json!({"node":config.node,"api_authenticated":true,"storage_active":true}))
}

pub fn apply(config: &Config, scope: &str) -> Result<Value> {
    root()?;
    config.validate()?;
    doctor(config)?;
    // Refuse collisions with unmanaged files before creating a credential or starting services.
    let mut effective = config.clone();
    if scope == "control" {
        effective.network.identity_ip = Some(access_ip(config)?);
    }
    let initial = plan::render(&effective, scope)?;
    plan::changes(config, &initial)?;
    let _lock = lock(config)?;
    plan::changes(config, &initial)?;
    for artifact in &initial {
        if artifact.capability == "network"
            && artifact.path.exists()
            && fs::read(&artifact.path)?.as_slice() != artifact.content.as_bytes()
        {
            return Err(Failure::action(
                "NETWORK_MIGRATION",
                "Changing an installed backplane requires a separate instance or an explicit network migration",
            ));
        }
    }
    for dir in [
        "runtime",
        "compute",
        "compute/config",
        "compute/storage",
        "compute/private",
        "compute/public",
        "access",
        "access/state",
        "access/dns",
        "control",
        "control/data",
        "control/config",
    ] {
        private_dir(&Path::new(&config.state_dir).join(dir))?;
    }
    if scope == "all" || scope == "compute" {
        let password = Path::new(&config.state_dir).join("compute/private/password");
        if !password.exists() {
            let mut random = [0u8; 32];
            getrandom::fill(&mut random)
                .map_err(|_| Failure::new("RANDOM", "OS randomness unavailable"))?;
            let value = random
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            atomic(&password, value.as_bytes(), 0o600)?;
        }
        private_password(config)?;
    }
    checkpoint(config, "prepared")?;
    let mut changed_units = Vec::new();
    for artifact in &initial {
        if ["dns", "control"].contains(&artifact.capability.as_str()) {
            continue;
        }
        if fs::read(&artifact.path).ok().as_deref() != Some(artifact.content.as_bytes()) {
            atomic(&artifact.path, artifact.content.as_bytes(), 0o644)?;
            if artifact.capability != "network" {
                changed_units.push(artifact.capability.as_str());
            }
        }
    }
    command(&["systemctl", "daemon-reload"], None, 30)?;
    checkpoint(config, "units_installed")?;
    for capability in ["compute", "access"] {
        if scope != "all" && scope != capability {
            continue;
        }
        let active = systemctl("is-active", config, capability).is_ok();
        if changed_units.contains(&capability) || !active {
            systemctl(if active { "restart" } else { "start" }, config, capability)?;
        }
        if capability == "compute" {
            let deadline = Instant::now() + Duration::from_secs(180);
            loop {
                if export_ca(config).is_ok() && compute_status(config).is_ok() {
                    break;
                }
                if Instant::now() >= deadline {
                    return Err(Failure::new(
                        "COMPUTE_STARTING",
                        "Compute is not ready after 180s; its state was preserved",
                    ));
                }
                thread::sleep(Duration::from_secs(3));
            }
        }
    }
    if scope != "compute" {
        effective.network.identity_ip = Some(access_ip(config)?);
        // Restore both namespace consumers after Access replacement when Control exists.
        let reconcile_scope = if scope == "access"
            && Path::new(&config.units_dir)
                .join(format!("{}.container", config.unit("control")))
                .exists()
        {
            "all"
        } else {
            scope
        };
        let full = plan::render(&effective, reconcile_scope)?;
        plan::changes(&effective, &full)?;
        // Validate proxy configuration before replacing its effective configuration.
        if let Some(caddy) = full.iter().find(|a| a.path.ends_with("Caddyfile")) {
            let candidate = Path::new(&config.state_dir).join("control/Caddyfile.candidate");
            atomic(&candidate, caddy.content.as_bytes(), 0o600)?;
            command(
                &[
                    "podman",
                    "run",
                    "--rm",
                    "--network=none",
                    "-v",
                    &format!("{}:/etc/caddy/Caddyfile:ro", candidate.display()),
                    "-v",
                    &format!(
                        "{}/compute/public/upstream-ca.crt:/etc/gnx/upstream-ca.crt:ro",
                        config.state_dir
                    ),
                    release::CONTROL_IMAGE,
                    "caddy",
                    "validate",
                    "--config",
                    "/etc/caddy/Caddyfile",
                    "--adapter",
                    "caddyfile",
                ],
                None,
                60,
            )?;
        }
        // Proxy first: removed names become inaccessible even while DNS is cached.
        for capability in ["control", "dns"] {
            let artifacts: Vec<_> = full.iter().filter(|a| a.capability == capability).collect();
            if artifacts.is_empty() {
                continue;
            }
            let changed = artifacts
                .iter()
                .any(|a| fs::read(&a.path).ok().as_deref() != Some(a.content.as_bytes()));
            for a in artifacts {
                if fs::read(&a.path).ok().as_deref() != Some(a.content.as_bytes()) {
                    atomic(&a.path, a.content.as_bytes(), 0o644)?;
                }
            }
            command(&["systemctl", "daemon-reload"], None, 30)?;
            let active = systemctl("is-active", config, capability).is_ok();
            if changed || !active {
                systemctl(if active { "restart" } else { "start" }, config, capability)?;
            }
        }
    }
    let result = status(&effective, scope)?;
    checkpoint(config, "applied")?;
    Ok(result)
}

pub fn status(config: &Config, scope: &str) -> Result<Value> {
    let mut result = json!({"revision":config.revision()});
    if scope == "all" || scope == "compute" {
        result["compute"] = compute_status(config)?;
    }
    if scope == "all" || scope == "access" {
        result["access"] = json!({"ip":access_ip(config)?});
    }
    if scope == "all" || scope == "control" {
        systemctl("is-active", config, "control")?;
        let ip = access_ip(config)?;
        let pid = command(
            &[
                "podman",
                "inspect",
                "--format",
                "{{.State.Pid}}",
                &config.unit("access"),
            ],
            None,
            10,
        )?;
        let pid = String::from_utf8_lossy(&pid).trim().to_string();
        if pid.parse::<u32>().unwrap_or(0) == 0 {
            return Err(Failure::new(
                "ACCESS_PID",
                "Access network namespace is missing",
            ));
        }
        let exe = std::env::current_exe()?;
        let ca =
            Path::new(&config.state_dir).join("control/data/caddy/pki/authorities/local/root.crt");
        command(
            &[
                "nsenter",
                "--target",
                &pid,
                "--net",
                exe.to_str()
                    .ok_or_else(|| Failure::new("PATH", "Binary path encoding"))?,
                "probe",
                "--ip",
                &ip.to_string(),
                "--ca",
                ca.to_str().unwrap(),
            ],
            None,
            30,
        )?;
        result["control"] =
            json!({"url":"https://compute.gnx","tls_verified":true,"dns_verified":true});
    }
    Ok(result)
}

pub fn probe(ip: Ipv4Addr, ca_path: &Path) -> Result<Value> {
    let mut query = vec![0x47, 0x4e, 0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in ["compute", "gnx"] {
        query.push(label.len() as u8);
        query.extend_from_slice(label.as_bytes());
    }
    query.extend_from_slice(&[0, 0, 1, 0, 1]);
    for tcp in [false, true] {
        let mut response = vec![0u8; 4096];
        let count = if tcp {
            let mut stream =
                TcpStream::connect_timeout(&SocketAddr::from((ip, 53)), Duration::from_secs(5))?;
            stream.set_read_timeout(Some(Duration::from_secs(5)))?;
            stream.write_all(&(query.len() as u16).to_be_bytes())?;
            stream.write_all(&query)?;
            let mut size = [0u8; 2];
            stream.read_exact(&mut size)?;
            let len = u16::from_be_bytes(size) as usize;
            if len > response.len() {
                return Err(Failure::new("DNS_RESPONSE", "DNS response too large"));
            }
            stream.read_exact(&mut response[..len])?;
            len
        } else {
            let socket = UdpSocket::bind("0.0.0.0:0")?;
            socket.set_read_timeout(Some(Duration::from_secs(5)))?;
            socket.connect((ip, 53))?;
            socket.send(&query)?;
            socket.recv(&mut response)?
        };
        if count < 12
            || response[..2] != query[..2]
            || response[2] & 0x84 != 0x84
            || response[3] & 15 != 0
            || u16::from_be_bytes([response[6], response[7]]) == 0
            || !response[..count].windows(4).any(|w| w == ip.octets())
        {
            return Err(Failure::new(
                "DNS_RESPONSE",
                "Expected authoritative compute.gnx IPv4 answer",
            ));
        }
    }
    let ca = reqwest::Certificate::from_pem(&fs::read(ca_path)?)
        .map_err(|_| Failure::new("CONTROL_CA", "Invalid Control CA"))?;
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .tls_built_in_root_certs(false)
        .add_root_certificate(ca)
        .resolve("compute.gnx", SocketAddr::from((ip, 443)))
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| Failure::new("CONTROL_TLS", "Cannot initialize Control TLS probe"))?;
    client
        .get("https://compute.gnx/")
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|_| Failure::new("CONTROL_TLS", "Compute HTTPS through Control failed"))?;
    Ok(json!({"dns_udp":true,"dns_tcp":true,"https":true}))
}
