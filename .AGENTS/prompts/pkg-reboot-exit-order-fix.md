# Corrección final PKG-02: orden del catch-all

Corrige únicamente el P2 restante en `installer/build.ps1`. No hagas commit ni push.

- Para cada paquete autorizado, exige que todos los `ExitCode` con `Value` aparezcan antes del único catch-all sin `Value`/`Behavior="error"`.
- Preferiblemente exige que ese catch-all sea el último hijo `ExitCode`; no cambies el bundle porque ya tiene el orden correcto.
- Conserva mappings exactos, constantes Rust, bloqueo de overrides y demás invariantes.
- Añade/ejecuta una prueba negativa que mueva el catch-all antes de los mappings en `PrepareWsl` y otra en `ValidateHost`; ambas deben fallar.
- Repite parseo PowerShell, guard positivo/negativos, `cargo fmt --all -- --check`, `cargo test --workspace`, build WiX completo, hashes/tamaños y `git diff --check`.

No amplíes alcance y no hagas commit/push.
