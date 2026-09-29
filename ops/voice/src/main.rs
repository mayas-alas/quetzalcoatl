//! Operator helper only: no LXC provisioning and no change to the public gnx CLI.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, &'static str>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema: u32,
    guest_hostname: String,
    tailscale_hostname: String,
    public_origin: String,
    voice_image: String,
    tailscale_image: String,
    hf_model: String,
    hf_tts_model: String,
    hf_tts_provider: String,
}
fn image(value: &str) -> bool {
    let Some((repo, digest)) = value.split_once("@sha256:") else {
        return false;
    };
    !repo.is_empty()
        && !repo.starts_with('-')
        && repo
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._/:-".contains(&c))
        && digest.len() == 64
        && digest
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn model(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._/-".contains(&c))
}
fn parse(source: &str) -> Result<Config> {
    let c: Config = toml::from_str(source).map_err(|_| "INVALID_CONFIG")?;
    if c.schema != 1
        || !gnx::domain::node::valid_name(&c.guest_hostname)
        || !gnx::domain::node::valid_name(&c.tailscale_hostname)
    {
        return Err("INVALID_IDENTITY");
    }
    if !image(&c.voice_image) || !image(&c.tailscale_image) {
        return Err("VERIFIED_IMAGE_DIGESTS_REQUIRED");
    }
    if c.public_origin != "https://voice.gnx" {
        return Err("GNX_VOICE_ORIGIN_REQUIRED");
    }
    if !model(&c.hf_model)
        || !model(&c.hf_tts_model)
        || !["fal-ai", "deepinfra", "hf-inference"].contains(&c.hf_tts_provider.as_str())
    {
        return Err("INVALID_MODEL_CONFIG");
    }
    Ok(c)
}
const PAYLOAD_FILES: [(&str, &str); 5] = [
    ("gnx-voice.pod", "gnx-voice.pod"),
    ("gnx-voice-state.volume", "gnx-voice-state.volume"),
    ("gnx-voice.container.in", "gnx-voice.container"),
    (
        "gnx-voice-access.container.in",
        "gnx-voice-access.container",
    ),
    ("serve.json", "serve.json"),
];
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u32,
    files: BTreeMap<String, String>,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn payload(dir: &Path, expected: &str) -> Result<BTreeMap<&'static str, String>> {
    // The expected manifest hash must come from the reviewed release channel,
    // not from an untrusted payload's own checksum file.
    let bytes = fs::read(dir.join("manifest.json")).map_err(|_| "PAYLOAD_READ_FAILED")?;
    if expected.len() != 64 || digest(&bytes) != expected {
        return Err("PAYLOAD_DIGEST_MISMATCH");
    }
    let manifest: Manifest =
        serde_json::from_slice(&bytes).map_err(|_| "PAYLOAD_MANIFEST_INVALID")?;
    if manifest.schema != 1 || manifest.files.len() != PAYLOAD_FILES.len() {
        return Err("PAYLOAD_MANIFEST_INVALID");
    }
    let mut files = BTreeMap::new();
    for (source, target) in PAYLOAD_FILES {
        let bytes = fs::read(dir.join(source)).map_err(|_| "PAYLOAD_READ_FAILED")?;
        if manifest.files.get(source) != Some(&digest(&bytes)) {
            return Err("PAYLOAD_FILE_MISMATCH");
        }
        files.insert(
            target,
            String::from_utf8(bytes).map_err(|_| "PAYLOAD_ENCODING_INVALID")?,
        );
    }
    Ok(files)
}
fn bundle(c: &Config, mut files: BTreeMap<&'static str, String>) -> BTreeMap<&'static str, String> {
    for content in files.values_mut() {
        *content = content.replace('\r', "");
        for (key, value) in [
            ("@VOICE_IMAGE@", &c.voice_image),
            ("@TAILSCALE_IMAGE@", &c.tailscale_image),
            ("@TAILSCALE_HOSTNAME@", &c.tailscale_hostname),
            ("@PUBLIC_ORIGIN@", &c.public_origin),
            ("@HF_MODEL@", &c.hf_model),
            ("@HF_TTS_MODEL@", &c.hf_tts_model),
            ("@HF_TTS_PROVIDER@", &c.hf_tts_provider),
        ] {
            *content = content.replace(key, value);
        }
    }
    files
}
fn render(files: &BTreeMap<&str, String>, output: &Path) -> Result<()> {
    // Refuse an existing target; never merge with another operator's bundle.
    fs::create_dir(output).map_err(|_| "NEW_OUTPUT_DIRECTORY_REQUIRED")?;
    for (name, content) in files {
        fs::write(output.join(name), content).map_err(|_| "BUNDLE_WRITE_FAILED")?;
    }
    Ok(())
}
fn command(program: &str, args: &[&str]) -> Result<String> {
    let out = gnx::adapter::process::checked(program, args, None, 10)
        .map_err(|_| "GUEST_CHECK_FAILED")?;
    Ok(String::from_utf8_lossy(&out).trim().to_owned())
}
fn check(c: &Config) -> Result<()> {
    if !cfg!(target_os = "linux") {
        return Err("LXC_LINUX_REQUIRED");
    }
    if command("id", &["-u"])? == "0" || command("id", &["-un"])? != "gnx-voice" {
        return Err("DEDICATED_ROOTLESS_USER_REQUIRED");
    }
    if command(
        "loginctl",
        &["show-user", "gnx-voice", "-p", "Linger", "--value"],
    )? != "yes"
    {
        return Err("USER_LINGER_REQUIRED");
    }
    command(
        "systemctl",
        &["--user", "show", "--property=Version", "--value"],
    )?;
    let home = service_home()?;
    private_secrets(&home)?;
    let detected = command("systemd-detect-virt", &["--container"])?;
    let marker = fs::read_to_string("/run/systemd/container")
        .unwrap_or_default()
        .trim()
        .to_owned();
    // A WSL kernel is detected before the nested LXC by systemd. The root-owned
    // systemd container marker remains the authoritative inner boundary.
    if detected != "lxc" && marker != "lxc" {
        return Err("LXC_REQUIRED");
    }
    if command("hostname", &[])? != c.guest_hostname {
        return Err("GUEST_IDENTITY_MISMATCH");
    }
    if !Path::new("/run/systemd/system").is_dir()
        || !Path::new("/sys/fs/cgroup/cgroup.controllers").is_file()
    {
        return Err("SYSTEMD_CGROUP_V2_REQUIRED");
    }
    gnx::adapter::podman::require_supported().map_err(|_| "PODMAN_6_QUADLET_REQUIRED")?;
    let info: serde_json::Value =
        serde_json::from_str(&command("podman", &["info", "--format", "json"])?)
            .map_err(|_| "PODMAN_INFO_INVALID")?;
    if info["host"]["security"]["rootless"] != true
        || info["host"]["cgroupVersion"] != "v2"
        || info["host"]["cgroupManager"] != "systemd"
    {
        return Err("ROOTLESS_SYSTEMD_CGROUP_REQUIRED");
    }
    for image in [&c.voice_image, &c.tailscale_image] {
        command("podman", &["image", "exists", image]).map_err(|_| "PINNED_IMAGES_REQUIRED")?;
    }
    Ok(())
}
fn service_home() -> Result<PathBuf> {
    let entry = command("getent", &["passwd", "gnx-voice"])?;
    let fields: Vec<_> = entry.split(':').collect();
    if fields.len() != 7
        || fields[2] == "0"
        || !["/usr/sbin/nologin", "/sbin/nologin"].contains(&fields[6])
    {
        return Err("DEDICATED_ACCOUNT_INVALID");
    }
    let home = PathBuf::from(fields[5]);
    if !home.is_absolute()
        || home == Path::new("/")
        || std::env::var_os("HOME").map(PathBuf::from) != Some(home.clone())
    {
        return Err("SERVICE_HOME_MISMATCH");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::symlink_metadata(&home).map_err(|_| "SERVICE_HOME_MISSING")?;
        if !metadata.is_dir()
            || metadata.uid().to_string() != fields[2]
            || metadata.mode() & 0o022 != 0
        {
            return Err("SERVICE_HOME_PERMISSIONS_INVALID");
        }
    }
    Ok(home)
}
fn private_secrets(home: &Path) -> Result<()> {
    let dir = home.join(".config/gnx/voice/secrets");
    for ancestor in dir.ancestors() {
        if ancestor
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err("SECRET_SYMLINK_REFUSED");
        }
    }
    for name in ["", "hf-token", "api-token", "enrollment-key"] {
        let path = if name.is_empty() {
            dir.clone()
        } else {
            dir.join(name)
        };
        let metadata = fs::symlink_metadata(path).map_err(|_| "SECRET_FILE_REQUIRED")?;
        if (name.is_empty() && !metadata.is_dir()) || (!name.is_empty() && !metadata.is_file()) {
            return Err("SECRET_FILE_TYPE_INVALID");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let owner = fs::metadata(home)
                .map_err(|_| "SERVICE_HOME_MISSING")?
                .uid();
            if metadata.uid() != owner || metadata.mode() & 0o077 != 0 {
                return Err("SECRET_PERMISSIONS_INVALID");
            }
        }
    }
    Ok(())
}
fn supply(c: &Config, files: &BTreeMap<&str, String>) -> Result<()> {
    check(c)?;
    let home = service_home()?;
    let config = home.join(".config");
    let destinations: Vec<_> = files
        .iter()
        .map(|(name, content)| {
            let dir = if *name == "serve.json" {
                config.join("gnx/voice")
            } else {
                config.join("containers/systemd")
            };
            (dir.join(name), content)
        })
        .collect();
    // Refuse any existing target (including symlinks); no adoption or overwrite.
    for (path, _) in &destinations {
        match path.symlink_metadata() {
            Ok(_) => return Err("EXISTING_CONFIGURATION_REFUSED"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("CONFIGURATION_READ_FAILED"),
        }
    }
    for name in [
        "gnx-voice",
        "gnx-voice-access",
        "gnx-voice-pod",
        "gnx-voice-state-volume",
    ] {
        for dir in [
            config.join("systemd/user"),
            PathBuf::from("/etc/systemd/user"),
            PathBuf::from("/usr/lib/systemd/user"),
            PathBuf::from("/lib/systemd/user"),
            PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").ok_or("USER_RUNTIME_REQUIRED")?)
                .join("systemd/user"),
        ] {
            match dir.join(format!("{name}.service")).symlink_metadata() {
                Ok(_) => return Err("EXISTING_SERVICE_REFUSED"),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err("CONFIGURATION_READ_FAILED"),
            }
        }
    }
    // Files only: never enroll, pull, start/stop services, alter DNS, or call pct.
    // After any interrupted write, refusal on retry requires operator review.
    for (path, content) in destinations {
        let parent = path.parent().ok_or("CONFIGURATION_PATH_INVALID")?;
        for ancestor in parent.ancestors() {
            if ancestor
                .symlink_metadata()
                .is_ok_and(|m| m.file_type().is_symlink())
            {
                return Err("CONFIGURATION_SYMLINK_REFUSED");
            }
        }
        fs::create_dir_all(parent).map_err(|_| "CONFIGURATION_WRITE_FAILED")?;
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o644);
        }
        let mut file = options
            .open(path)
            .map_err(|_| "CONFIGURATION_WRITE_FAILED")?;
        file.write_all(content.as_bytes())
            .map_err(|_| "CONFIGURATION_WRITE_FAILED")?;
        file.sync_all().map_err(|_| "CONFIGURATION_WRITE_FAILED")?;
    }
    Ok(())
}
fn observed_route(c: &Config) -> Result<gnx::config::Route> {
    check(c)?;
    let text = command(
        "podman",
        &["exec", "gnx-voice-access", "tailscale", "status", "--json"],
    )?;
    route_from_status(&text)
}
fn route_from_status(text: &str) -> Result<gnx::config::Route> {
    let status: serde_json::Value =
        serde_json::from_str(text).map_err(|_| "ACCESS_STATUS_INVALID")?;
    if status["BackendState"] != "Running" || status["Self"]["Online"] != true {
        return Err("ACCESS_ENROLLMENT_REQUIRED");
    }
    let ip = status["Self"]["TailscaleIPs"]
        .as_array()
        .and_then(|ips| {
            ips.iter()
                .filter_map(|ip| ip.as_str()?.parse::<std::net::Ipv4Addr>().ok())
                .find(|ip| ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
        })
        .ok_or("PRIVATE_IDENTITY_REQUIRED")?;
    let route = gnx::config::Route {
        hostname: "voice.gnx".into(),
        upstream: format!("http://{ip}:8080"),
    };
    gnx::domain::control::validate(std::slice::from_ref(&route)).map_err(|_| "ROUTE_INVALID")?;
    Ok(route)
}
fn run() -> Result<serde_json::Value> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err("ARGUMENTS_REQUIRED");
    }
    let source = fs::read_to_string(&args[1]).map_err(|_| "CONFIGURATION_READ_FAILED")?;
    let c = parse(&source)?;
    match (args[0].as_str(), args.len()) {
        ("render", 5) => {
            let files = bundle(&c, payload(Path::new(&args[2]), &args[3])?);
            render(&files, Path::new(&args[4]))?;
            Ok(serde_json::json!({"state":"STAGED", "code":"RENDERED_NOT_DEPLOYED"}))
        }
        ("check", 2) => {
            check(&c)?;
            Ok(serde_json::json!({"state":"STAGED", "code":"PREREQUISITES_CHECKED_NOT_READY"}))
        }
        ("supply", 4) => {
            let files = bundle(&c, payload(Path::new(&args[2]), &args[3])?);
            supply(&c, &files)?;
            Ok(serde_json::json!({"state":"STAGED", "code":"CONFIGURATION_SUPPLIED_NOT_STARTED"}))
        }
        ("route", 2) => Ok(
            serde_json::json!({"state":"STAGED", "code":"ROUTE_OBSERVED_NOT_PUBLISHED", "routes":[observed_route(&c)?]}),
        ),
        _ => Err("ARGUMENTS_REQUIRED"),
    }
}
fn main() {
    match run() {
        Ok(report) => println!("{report}"),
        Err(code) => {
            println!(
                "{}",
                serde_json::json!({"state":"ACTION_REQUIRED", "code":code})
            );
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn recipe() -> String {
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/voice.toml"))
            .unwrap()
    }
    fn templates() -> BTreeMap<&'static str, String> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime/voice");
        PAYLOAD_FILES
            .into_iter()
            .map(|(source, target)| (target, fs::read_to_string(dir.join(source)).unwrap()))
            .collect()
    }
    #[test]
    fn external_payload_is_pinned_and_tamper_is_rejected_before_render() {
        let dir = std::env::temp_dir().join(format!("gnx-voice-payload-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        let mut hashes = BTreeMap::new();
        let templates = templates();
        for (source, target) in PAYLOAD_FILES {
            let bytes = templates[target].as_bytes();
            fs::write(dir.join(source), bytes).unwrap();
            hashes.insert(source, digest(bytes));
        }
        let bytes = serde_json::to_vec(&serde_json::json!({"schema":1,"files":hashes})).unwrap();
        fs::write(dir.join("manifest.json"), &bytes).unwrap();
        let expected = digest(&bytes);
        assert_eq!(payload(&dir, &expected).unwrap(), templates);
        assert_eq!(
            payload(&dir, &"0".repeat(64)).unwrap_err(),
            "PAYLOAD_DIGEST_MISMATCH"
        );
        fs::write(dir.join("serve.json"), "{}").unwrap();
        assert_eq!(
            payload(&dir, &expected).unwrap_err(),
            "PAYLOAD_FILE_MISMATCH"
        );
        fs::remove_file(dir.join("serve.json")).unwrap();
        assert_eq!(payload(&dir, &expected).unwrap_err(), "PAYLOAD_READ_FAILED");
        hashes.insert("../unexpected", "0".repeat(64));
        let bad = serde_json::to_vec(&serde_json::json!({"schema":1,"files":hashes})).unwrap();
        fs::write(dir.join("manifest.json"), &bad).unwrap();
        assert_eq!(
            payload(&dir, &digest(&bad)).unwrap_err(),
            "PAYLOAD_MANIFEST_INVALID"
        );
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn private_example_uses_one_portal_and_private_gateway_bindings() {
        let text = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/gnx.private.example.toml"),
        )
        .unwrap();
        let c = gnx::config::Config::parse(&text).unwrap();
        let gateway = gnx::adapter::caddy::render(&c, "100.64.0.10", "gnx-compute", "10.90.0.2");
        assert_eq!(gateway.matches("https://app.gnx").count(), 1);
        assert_eq!(gateway.matches("https://voice.gnx").count(), 1);
        assert_eq!(gateway.matches("https://computer.gnx").count(), 1);
        assert!(!gateway.contains("127.0.0.1"));
        assert!(!gateway.contains("0.0.0.0"));
        assert!(gateway.contains("reverse_proxy http://100.64.0.20:8080"));
        let dns = gnx::adapter::coredns::zone(&c, "100.64.0.10");
        assert!(dns.contains("app IN A 100.64.0.10"));
        assert!(dns.contains("voice IN A 100.64.0.10"));
        assert!(dns.contains("computer IN A 100.64.0.10"));
        assert!(dns.contains("proxmox IN A 100.64.0.10"));
        let mut override_app = c.clone();
        override_app.routes.push(gnx::config::Route {
            hostname: "app.gnx".into(),
            upstream: "http://100.64.0.30:8080".into(),
        });
        let custom =
            gnx::adapter::caddy::render(&override_app, "100.64.0.10", "gnx-compute", "10.90.0.2");
        assert_eq!(custom.matches("https://app.gnx").count(), 1);
        assert!(!custom.contains("root * /gnx-app"));
        assert_eq!(
            gnx::adapter::coredns::zone(&override_app, "100.64.0.10")
                .matches("app IN A")
                .count(),
            1
        );
        for name in ["computer.gnx", "proxmox.gnx", "ns.gnx"] {
            let route = gnx::config::Route {
                hostname: name.into(),
                upstream: "http://100.64.0.30:8080".into(),
            };
            assert!(gnx::domain::control::validate(&[route]).is_err());
        }
    }
    fn source() -> String {
        recipe()
            .replace(
                "voice_image = \"\"",
                &format!(
                    "voice_image = \"registry.example/voice@sha256:{}\"",
                    "a".repeat(64)
                ),
            )
            .replace(
                "tailscale_image = \"\"",
                &format!(
                    "tailscale_image = \"registry.example/access@sha256:{}\"",
                    "b".repeat(64)
                ),
            )
    }
    #[test]
    fn rejects_unselected_release_and_unsafe_inputs() {
        assert!(parse(&recipe()).is_err());
        for bad in [
            source().replace("https://voice.gnx", "https://elsewhere.invalid"),
            source().replace("https://", "http://"),
            source().replace(&"a".repeat(64), "latest"),
            format!("{}\nhf_token='not-allowed'", source()),
            source().replace("fal-ai", "unknown"),
        ] {
            assert!(parse(&bad).is_err());
        }
        assert!(!image("-option@sha256:abc"));
    }
    #[test]
    fn uses_existing_control_and_dns_with_observed_private_identity() {
        let status = serde_json::json!({"BackendState":"Running", "Self":{"Online":true,"TailscaleIPs":["100.64.0.20"]}});
        let route = route_from_status(&status.to_string()).unwrap();
        let mut core = gnx::config::Config::parse(
            "schema=1\ninstance='gnx'\nnode='compute'\n[network]\nsubnet='10.90.0.0/24'\n",
        )
        .unwrap();
        core.routes.push(route);
        let gateway = gnx::adapter::caddy::render(&core, "100.64.0.10", "gnx-compute", "10.90.0.2");
        assert!(gateway.contains("https://voice.gnx"));
        assert!(gateway.contains("reverse_proxy http://100.64.0.20:8080"));
        let dns = gnx::adapter::coredns::zone(&core, "100.64.0.10");
        assert!(dns.contains("voice IN A 100.64.0.10"));
        assert!(!dns.contains("100.64.0.20"));
        let mut offline = status.clone();
        offline["Self"]["Online"] = false.into();
        assert!(route_from_status(&offline.to_string()).is_err());
        let mut public = status;
        public["Self"]["TailscaleIPs"] = serde_json::json!(["8.8.8.8"]);
        assert!(route_from_status(&public.to_string()).is_err());
    }
    #[test]
    fn private_sidecar_and_file_secrets_are_explicit() {
        let files = bundle(&parse(&source()).unwrap(), templates());
        assert_eq!(files.len(), 5);
        let app = &files["gnx-voice.container"];
        let access = &files["gnx-voice-access.container"];
        assert!(app.contains("BIND_HOST=127.0.0.1"));
        assert!(app.contains("HF_TOKEN_FILE=/run/secrets/hf-token"));
        assert!(app.contains("GNX_API_TOKEN_FILE=/run/secrets/api-token"));
        assert!(access.contains("TS_USERSPACE=true"));
        assert!(access.contains("TS_AUTHKEY=file:/run/secrets/enrollment-key"));
        for unit in [app, access] {
            assert!(unit.contains("Pod=gnx-voice.pod"));
            assert!(unit.contains("WantedBy=default.target"));
            assert!(!unit.contains("/etc/gnx/voice"));
            assert!(!unit.contains("PublishPort="));
            assert!(!unit.contains("Network=host"));
            assert!(!unit.contains("Privileged=true"));
        }
        let serve: serde_json::Value = serde_json::from_str(&files["serve.json"]).unwrap();
        assert!(serve.get("AllowFunnel").is_none());
        assert!(serve.get("Web").is_none());
        assert_eq!(serve["TCP"]["8080"]["TCPForward"], "127.0.0.1:8080");
        assert!(app.contains("APP_ORIGIN=https://voice.gnx"));
    }
    #[test]
    fn render_never_overwrites_and_does_not_enroll() {
        let dir = std::env::temp_dir().join(format!("gnx-voice-recipe-{}", std::process::id()));
        let c = parse(&source()).unwrap();
        let files = bundle(&c, templates());
        render(&files, &dir).unwrap();
        assert_eq!(
            render(&files, &dir).unwrap_err(),
            "NEW_OUTPUT_DIRECTORY_REQUIRED"
        );
        assert!(fs::read_to_string(dir.join("serve.json"))
            .unwrap()
            .contains("TCPForward"));
        fs::remove_dir_all(dir).unwrap();
    }
}
