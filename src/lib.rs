// Shared by both executables. Installed names are stable across versions.
pub const PRODUCT: &str = "Quetzalcoatl GNX";
pub const CLI_EXE: &str = "quetzalcoatl-gnx.exe";
pub const SETUP_EXE: &str = "quetzalcoatl-gnx-setup.exe";
pub const TRAY_EXE: &str = "quetzalcoatl-gnx-tray.exe";
/// Short interactive command installed beside the canonical CLI.
pub const GNX_EXE: &str = "gnx.exe";

pub mod protocol;
pub mod installer;

/// Proposed architecture boundaries. Implementations remain in the existing
/// small modules during the incremental migration.
pub mod core {
    pub use crate::{installer::{Stage, State}, protocol::{Operation, Report}};
}
#[cfg(windows)]
pub mod platform {
    pub use crate::windows::installer;
}
#[cfg(windows)]
pub mod windows;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_names_match_manifest() {
        let manifest = include_str!("../Cargo.toml");
        for exe in [CLI_EXE, SETUP_EXE] {
            assert!(manifest.contains(&format!("name = \"{}\"", exe.trim_end_matches(".exe"))));
        }
    }
}
