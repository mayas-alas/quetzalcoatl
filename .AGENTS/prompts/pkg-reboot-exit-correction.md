# Corrección PKG-02: reinicio coherente en toda la cadena Burn

Corrige únicamente los hallazgos de la revisión PKG-02 en el repositorio actual. No hagas commit ni push. Mantén el alcance PoC/MVP.

Hallazgos confirmados:

- `scheduleReboot` no corta la cadena; después de `PrepareWsl`, Burn continúa por WSL, Podman y `ValidateHost`.
- `ValidateHost` ejecuta el mismo helper sin `prepare-wsl`; su check `pending_reboot` devuelve `REBOOT_PENDING=14` y su catch-all vuelve a producir `0x8007000e`.
- El guard actual verifica solo `PrepareWsl` y no impone una lista cerrada de paquetes autorizados a tratar 14 como reinicio.

Implementación requerida:

1. Mantén `Value="14" Behavior="scheduleReboot"` en `PrepareWsl` y añádelo a `ValidateHost`, siempre antes de su catch-all `Behavior="error"`. Conserva 3010 en `PrepareWsl`.
2. No conviertas ningún otro código (10–13, 15–16, 20, 64) en éxito o reinicio.
3. Generaliza el guard de `installer/build.ps1` para leer `REBOOT_PENDING` desde Rust y exigir el mapping, en orden correcto, exactamente en los dos `ExePackage` autorizados: `PrepareWsl` y `ValidateHost`.
4. El guard también debe recorrer todos los `ExePackage` del bundle y fallar si cualquier paquete fuera de esa allowlist contiene el mapping del valor Rust a `scheduleReboot`. Debe seguir detectando paquete ausente/duplicado, mapping ausente y catch-all ausente/antes del mapping; evita regex codiciosa entre paquetes.
5. Actualiza la frase técnica de `docs/VALIDATION.md` para explicar que ambos pasos aceptan exclusivamente `REBOOT_PENDING=14`; sigue aclarando que Dockur no cierra G5.

Validaciones obligatorias:

- Parseo PowerShell.
- Pruebas negativas del guard sobre copias temporales o una función invocable: paquete autorizado sin mapping, mapping después de catch-all y mapping agregado a un tercer ExePackage deben fallar. No dejes archivos temporales en el repo.
- `cargo fmt --all -- --check` y `cargo test --workspace`.
- Build completo `installer/build.ps1`/WiX.
- SHA-256 y tamaño de Setup/MSI nuevos.
- `git diff --check`.

No modifiques otros componentes y no hagas commit ni push.
