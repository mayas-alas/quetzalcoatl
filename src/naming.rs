//! Fixed product identifiers. Changing these after installation requires migration.
pub const PRODUCT: &str = "Quetzalcoatl GNX";
pub const CLI_EXE: &str = "quetzalcoatl-gnx.exe";
pub const SETUP_EXE: &str = "quetzalcoatl-gnx-setup.exe";
pub const SERVICE: &str = "QuetzalcoatlGNX";
pub const SERVICE_DISPLAY: &str = "Quetzalcoatl GNX - Consultas WSL";
pub const ACCOUNT: &str = "svc_quetzalcoatl_gnx";
pub const DISTRO: &str = "quetzalcoatl-gnx";
pub const LINUX_USER: &str = "quetzalcoatl-gnx";
pub const PIPE: &str = r"\\.\pipe\quetzalcoatl-gnx-control-v1";
pub const BASE: &str = r"C:\ProgramData\QuetzalcoatlGNX";

pub fn bootstrap() -> String {
    include_str!("bootstrap.sh").replace("@LINUX_USER@", LINUX_USER).replace('\r', "")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_match_packaging_and_windows_limits() {
        let manifest = include_str!("../Cargo.toml");
        for exe in [CLI_EXE, SETUP_EXE] {
            assert!(manifest.contains(&format!("name = \"{}\"", exe.trim_end_matches(".exe"))));
            assert!(exe.starts_with("quetzalcoatl-gnx"));
        }
        assert!(ACCOUNT.len() <= 20);
        assert!(ACCOUNT.bytes().all(|c| c.is_ascii_lowercase() || c == b'_'));
        assert!(PIPE.ends_with("quetzalcoatl-gnx-control-v1"));
    }
    #[test]
    fn linux_identity_is_resolved_consistently() {
        let script = bootstrap();
        assert!(!script.contains("@LINUX_USER@"));
        assert!(!script.contains('\r'));
        assert!(script.contains(&format!("default={LINUX_USER}")));
        assert!(script.contains(&format!("/home/{LINUX_USER}/.config/containers/systemd")));
        assert!(script.contains(&format!("/var/lib/systemd/linger/{LINUX_USER}")));
    }
}
