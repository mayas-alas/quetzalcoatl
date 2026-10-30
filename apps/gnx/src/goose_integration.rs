// goos_integration.rs — integración goose-driven para usuario host Windows
// No aplica al servicio `GnXCore` ni a la cuenta dedicada `gnxcoresvc`.
// Paradigma: usar `goose.exe` (CLI instalada) como framework de skills.

pub const GOOSE_CLI_PATH: &str = r"C:\Program Files\Goose-win32-x64\dist-windows\resources\bin\goose.exe";
pub const SKILLS_LOCAL_DIR: &str = r"C:\Users\mayas\.agents\skills";
pub const SKILLS_REPO_DIR: &str = r".opencode\skills";

/// Documenta que el paso debe ser ejecutado por el usuario host,
/// no por `gnxcoresvc`, siguiendo `.AGENTS` y `scope.md`.
pub fn verify_host_context() -> &'static str {
    "host-user-only: no service-account"
}
