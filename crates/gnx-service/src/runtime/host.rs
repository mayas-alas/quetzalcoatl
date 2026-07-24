use super::*;

pub(super) fn validate_identity() -> Result<PathBuf, GateError> {
    let whoami = system_binary("whoami.exe")
        .map_err(|error| error.with_code("RUNTIME_IDENTITY_INVALID", Component::None))?;
    let output = run_command(&whoami, ["/user", "/fo", "csv", "/nh"])
        .map_err(|error| error.with_code("RUNTIME_IDENTITY_INVALID", Component::None))?;
    let identity = String::from_utf8_lossy(&output.stdout);
    if !identity.contains(EXPECTED_SERVICE_SID) {
        return Err(GateError::new(
            "RUNTIME_IDENTITY_INVALID",
            Component::None,
            "service process is not running under NT SERVICE\\Quetzalcoatl",
        ));
    }

    let profile = env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.is_dir())
        .ok_or_else(|| {
            GateError::new(
                "RUNTIME_IDENTITY_INVALID",
                Component::None,
                "SCM did not load an absolute service profile",
            )
        })?;
    Ok(profile)
}

pub(super) fn configure_wsl(profile: &Path) -> Result<(), GateError> {
    let wsl = system_binary("wsl.exe")
        .map_err(|error| error.with_code("WSL_NESTED_VIRT_FAILED", Component::Wsl))?;
    run_command(&wsl, ["--version"])
        .map_err(|error| error.with_code("WSL_NESTED_VIRT_FAILED", Component::Wsl))?;

    let config = profile.join(".wslconfig");
    let changed = fs::read(&config).map_or(true, |current| current != WSL_CONFIG.as_bytes());
    if changed {
        fs::write(&config, WSL_CONFIG).map_err(|error| {
            GateError::new(
                "WSL_NESTED_VIRT_FAILED",
                Component::Wsl,
                format!("cannot write managed .wslconfig: {error}"),
            )
        })?;
        if fs::read(&config).ok().as_deref() != Some(WSL_CONFIG.as_bytes()) {
            return Err(GateError::new(
                "WSL_NESTED_VIRT_FAILED",
                Component::Wsl,
                "managed .wslconfig did not round-trip",
            ));
        }
        run_command(&wsl, ["--shutdown"])
            .map_err(|error| error.with_code("WSL_NESTED_VIRT_FAILED", Component::Wsl))?;
    }
    Ok(())
}
