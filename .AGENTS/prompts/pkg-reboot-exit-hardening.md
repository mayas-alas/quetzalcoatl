# Hardening final PKG-02: guard no evadible y mappings exactos

Corrige únicamente los dos hallazgos de la revisión final en el repositorio actual. No hagas commit ni push.

Hallazgos:

1. `-RebootContractBundleXml`/`-RebootContractBundlePath` pueden validar XML alterno y luego permitir que el build compile el `bundle.wxs` real sin validarlo.
2. El guard cuenta solo mappings 14 con `scheduleReboot`; un segundo `Value=14` con otro comportamiento pasaría.

Implementación requerida:

- Rechaza cualquier `RebootContractBundleXml` o `RebootContractBundlePath` salvo que también se especifique `-TestRebootContractOnly`. En build normal, el guard debe validar siempre y únicamente `installer/bundle.wxs`, que es el archivo compilado.
- Lee desde `crates/host-preflight/src/exit_codes.rs` tanto `REBOOT_PENDING` como `REBOOT_REQUIRED`; no dupliques 14/3010 sin comprobarlos contra Rust.
- Exige conjuntos explícitos exactos por paquete:
  - `PrepareWsl`: `0 -> success`, `REBOOT_PENDING -> scheduleReboot`, `REBOOT_REQUIRED -> scheduleReboot` y exactamente un catch-all sin `Value` con `error`.
  - `ValidateHost`: `0 -> success`, `REBOOT_PENDING -> scheduleReboot` y exactamente un catch-all sin `Value` con `error`.
- No permitas otro `ExitCode` con `Value` en esos paquetes ni duplicados de ningún valor; así 10–13, 15–16, 20 y 64 necesariamente usan el catch-all.
- Conserva el rechazo de mapping `REBOOT_PENDING -> scheduleReboot` en cualquier tercer `ExePackage`.
- Mantén la documentación separada de G5.

Valida:

- Parseo PowerShell y guard positivo.
- Negativos existentes más: duplicado 14 con `success`; código valorado adicional; 3010 ausente/equivocado; override XML/path sin `-TestRebootContractOnly`. Todos deben fallar.
- `cargo fmt --all -- --check`, `cargo test --workspace`, build WiX completo, hashes/tamaños y `git diff --check`.

No amplíes alcance y no hagas commit/push.
