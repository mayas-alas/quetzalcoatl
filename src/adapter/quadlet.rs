//! Technical lifecycle adapter; public GNX capability and unit names stay unchanged.
use std::path::Path;

pub const OWNER: &str = "# Managed by GNX Quadlet v1";
pub const CAPABILITIES: [&str; 4] = ["compute", "access", "dns", "control"];

pub fn render(instance: &str, cap: &str, args: &str, requires: &str) -> Result<String, String> {
    if !crate::domain::node::valid_name(instance) || !CAPABILITIES.contains(&cap) {
        return Err("RUNTIME_UNIT_INVALID".into());
    }
    // Internal release arguments are whitespace-delimited, not shell syntax.
    // Reject systemd expansion/line injection instead of silently reinterpreting it.
    if args
        .chars()
        .any(|c| c.is_control() || "\"'\\%$".contains(c))
    {
        return Err("RUNTIME_ARGUMENT_INVALID".into());
    }
    let dependency = format!("gnx-{instance}-access.service");
    if !requires.is_empty() && requires != dependency {
        return Err("RUNTIME_DEPENDENCY_INVALID".into());
    }
    let tokens: Vec<_> = args.split_whitespace().collect();
    let image_index = tokens
        .iter()
        .position(|token| token.contains("@sha256:"))
        .ok_or("CONTAINER_IMAGE_MISSING")?;
    let image = tokens[image_index];
    let (repository, digest) = image
        .split_once("@sha256:")
        .ok_or("CONTAINER_IMAGE_INVALID")?;
    if repository.is_empty()
        || repository.starts_with('-')
        || digest.len() != 64
        || !digest
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err("CONTAINER_IMAGE_INVALID".into());
    }
    let podman_args = tokens[..image_index].join(" ");
    let exec = tokens[image_index + 1..].join(" ");
    let dependency_lines = if requires.is_empty() {
        String::new()
    } else {
        format!("Requires={requires}\nAfter={requires}\n")
    };
    let exec_line = if exec.is_empty() {
        String::new()
    } else {
        format!("Exec={exec}\n")
    };
    Ok(format!("{OWNER}\n[Unit]\nDescription=GNX {cap} ({instance})\nAfter=network-online.target\nWants=network-online.target\n{dependency_lines}StartLimitIntervalSec=300\nStartLimitBurst=5\n\n[Container]\nImage={image}\nContainerName=gnx-{instance}-{cap}\nPull=never\nPodmanArgs={podman_args}\n{exec_line}LogDriver=none\n\n[Service]\nRestart=on-failure\nRestartSec=5\nTimeoutStartSec=300\nTimeoutStopSec=120\nStandardOutput=null\nStandardError=null\n\n[Install]\nWantedBy=multi-user.target\n"))
}

/// Fail before mutation if another unit would shadow the generated service.
/// No adoption, deletion, stop, disable or implicit migration of an existing unit.
pub fn check_paths(quadlet: &Path, service_paths: &[&Path]) -> Result<(), String> {
    for path in service_paths {
        match path.symlink_metadata() {
            Ok(_) => return Err("QUADLET_MIGRATION_REQUIRED".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("RUNTIME_UNIT_READ_FAILED".into()),
        }
    }
    match quadlet.symlink_metadata() {
        Ok(m) if !m.is_file() || m.file_type().is_symlink() => Err("RUNTIME_UNIT_CONFLICT".into()),
        Ok(_) => {
            let existing =
                std::fs::read_to_string(quadlet).map_err(|_| "RUNTIME_UNIT_READ_FAILED")?;
            if existing.lines().next() == Some(OWNER) {
                Ok(())
            } else {
                Err("RUNTIME_UNIT_CONFLICT".into())
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("RUNTIME_UNIT_READ_FAILED".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args() -> String {
        format!("--network gnx-test --volume /var/lib/gnx/test/data:/data registry.example/compute@sha256:{} /sbin/init", "a".repeat(64))
    }
    #[test]
    fn preserves_identity_storage_digest_and_boot_lifecycle() {
        let unit = render("test", "compute", &args(), "").unwrap();
        for expected in [
            OWNER,
            "ContainerName=gnx-test-compute",
            "Pull=never",
            "Restart=on-failure",
            "StartLimitBurst=5",
            "WantedBy=multi-user.target",
            "Exec=/sbin/init",
            "--volume /var/lib/gnx/test/data:/data",
            "StandardOutput=null",
        ] {
            assert!(unit.contains(expected), "{expected}");
        }
        assert!(!unit.contains("ExecStart="));
        assert!(!unit.contains("Requires=\n"));
        assert_eq!(unit, render("test", "compute", &args(), "").unwrap());
    }
    #[test]
    fn keeps_private_network_dependency() {
        let unit = render("test", "dns", &args(), "gnx-test-access.service").unwrap();
        assert!(unit.contains("Requires=gnx-test-access.service\nAfter=gnx-test-access.service"));
    }
    #[test]
    fn refuses_unpinned_images_injection_and_unknown_names() {
        for bad in [
            "example:latest".to_owned(),
            args().replace(&"a".repeat(64), "bad"),
            format!("{}\n[Service]", args()),
            format!("{} $SECRET", args()),
            format!("{} %n", args()),
        ] {
            assert!(render("test", "compute", &bad, "").is_err());
        }
        assert!(render("../other", "compute", &args(), "").is_err());
        assert!(render("test", "unknown", &args(), "").is_err());
        assert!(render("test", "compute", &args(), "other.service").is_err());
    }
    #[test]
    fn refuses_existing_services_and_foreign_quadlets_without_changing_files() {
        let dir = std::env::temp_dir().join(format!("gnx-quadlet-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let quadlet = dir.join("test.container");
        let service = dir.join("test.service");
        assert!(check_paths(&quadlet, &[&service]).is_ok());
        std::fs::write(&service, "existing service").unwrap();
        assert_eq!(
            check_paths(&quadlet, &[&service]).unwrap_err(),
            "QUADLET_MIGRATION_REQUIRED"
        );
        assert_eq!(
            std::fs::read_to_string(&service).unwrap(),
            "existing service"
        );
        assert!(!quadlet.exists());
        std::fs::remove_file(&service).unwrap();
        std::fs::write(&quadlet, "foreign unit").unwrap();
        assert_eq!(
            check_paths(&quadlet, &[]).unwrap_err(),
            "RUNTIME_UNIT_CONFLICT"
        );
        assert_eq!(std::fs::read_to_string(&quadlet).unwrap(), "foreign unit");
        std::fs::write(&quadlet, render("test", "compute", &args(), "").unwrap()).unwrap();
        assert!(check_paths(&quadlet, &[]).is_ok());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
