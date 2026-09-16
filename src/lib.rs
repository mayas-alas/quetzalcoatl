// Shared by both executables. Installed names are stable across versions.
pub const PRODUCT: &str = "Quetzalcoatl GNX";
pub const CLI_EXE: &str = "quetzalcoatl-gnx.exe";
pub const SETUP_EXE: &str = "quetzalcoatl-gnx-setup.exe";

pub mod protocol;
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
