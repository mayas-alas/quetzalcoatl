//! Destructive package cleanup invoked only by MSI uninstall or a genuinely
//! fresh install that finds residue from a pre-cleanup release.

#[cfg(windows)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let state_root = crate::state::state_dir()
        .parent()
        .ok_or("state directory has no product parent")?
        .to_path_buf();
    let credential = crate::state::state_dir().join("identity/password.dpapi");
    let had_product_residue = state_root.exists() || hosts_contains_gnx_marker()?;

    // The machine-protected credential is the ownership marker for both the
    // fixed account and its per-user distro registration. WSL stores that
    // registration and VHDX below the dedicated profile; deleting the entire
    // product-owned profile removes both without launching a child under an
    // identity that is itself being destroyed. Never delete an unrelated
    // same-named local account found on a genuinely clean host.
    if credential.is_file() {
        crate::dedicated_account::remove_managed_identity()?;
    }

    if had_product_residue {
        remove_hosts_entries()?;
        remove_legacy_portproxy();
    }
    if state_root.exists() {
        std::fs::remove_dir_all(&state_root)?;
    }
    if let Some(parent) = state_root.parent() {
        let _ = std::fs::remove_dir(parent);
    }
    Ok(())
}

#[cfg(windows)]
fn hosts_path() -> std::path::PathBuf {
    std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"))
        .join(r"System32\drivers\etc\hosts")
}

#[cfg(windows)]
fn is_gnx_hosts_line(line: &str) -> bool {
    let content = line.split('#').next().unwrap_or_default();
    let names = content
        .split_whitespace()
        .skip(1)
        .map(|name| name.to_ascii_lowercase())
        .collect::<Vec<_>>();
    line.to_ascii_lowercase().contains("# gnx node")
        || names
            .iter()
            .any(|name| matches!(name.as_str(), "app.gnx" | "compute.gnx" | "agents.gnx"))
}

#[cfg(windows)]
fn hosts_contains_gnx_marker() -> std::io::Result<bool> {
    match std::fs::read_to_string(hosts_path()) {
        Ok(text) => Ok(text.lines().any(is_gnx_hosts_line)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(windows)]
fn remove_hosts_entries() -> std::io::Result<()> {
    let path = hosts_path();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let retained = text
        .lines()
        .filter(|line| !is_gnx_hosts_line(line))
        .collect::<Vec<_>>()
        .join("\r\n");
    let ending = if retained.is_empty() { "" } else { "\r\n" };
    std::fs::write(path, format!("{retained}{ending}"))
}

#[cfg(windows)]
fn remove_legacy_portproxy() {
    // Older GnX releases owned this exact loopback rule. Ignore "not found"
    // so cleanup remains idempotent on hosts that never had that release.
    let _ = std::process::Command::new("netsh")
        .args([
            "interface",
            "portproxy",
            "delete",
            "v4tov4",
            "listenaddress=127.0.0.1",
            "listenport=443",
        ])
        .status();
}

#[cfg(test)]
mod tests {
    #[test]
    fn recognizes_only_product_host_names() {
        assert!(super::is_gnx_hosts_line(
            "127.0.0.1 app.gnx compute.gnx # GnX Node"
        ));
        assert!(super::is_gnx_hosts_line("127.0.0.1 agents.gnx"));
        assert!(!super::is_gnx_hosts_line("127.0.0.1 example.test"));
        assert!(!super::is_gnx_hosts_line("# app.gnx documentation"));
    }
}
