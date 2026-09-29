use super::process;

fn supported(version: &str) -> bool {
    let mut parts = version.trim().split('.');
    let Some(major) = parts.next().and_then(|v| v.parse::<u32>().ok()) else {
        return false;
    };
    major >= 6
        && parts
            .next()
            .is_some_and(|v| !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit()))
}

pub fn require_supported() -> Result<(), String> {
    let version = process::checked(
        "podman",
        &["version", "--format", "{{.Client.Version}}"],
        None,
        10,
    )
    .map_err(|_| "PODMAN_6_REQUIRED")?;
    if !supported(&String::from_utf8_lossy(&version)) {
        return Err("PODMAN_6_REQUIRED".into());
    }
    // Both common distribution layouts; do not assume the engine package includes Quadlet.
    if ![
        "/usr/lib/systemd/system-generators/podman-system-generator",
        "/usr/libexec/podman/quadlet",
        "/usr/lib/podman/quadlet",
        "/lib/systemd/system-generators/podman-system-generator",
    ]
    .iter()
    .any(|path| std::path::Path::new(path).is_file())
    {
        return Err("QUADLET_GENERATOR_REQUIRED".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_six_and_future_major_versions_not_just_single_digits() {
        for value in ["6.0.0", "6.1.0-dev", "10.0.1", " 12.3.0\n"] {
            assert!(supported(value), "{value}");
        }
        for value in [
            "",
            "5.9.0",
            "podman version 6.0.0",
            "6",
            "6.x.0",
            "600garbage",
        ] {
            assert!(!supported(value), "{value}");
        }
    }
}
