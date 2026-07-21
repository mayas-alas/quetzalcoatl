# PKG-02: mapear REBOOT_PENDING de preflight en Burn

Implementa una corrección estrecha en el repositorio actual. No hagas commit ni push. No cambies casos de uso ni el contrato funcional del PoC/MVP.

Evidencia real en Windows 11 limpio dentro de Dockur:

- El bootstrapper abre correctamente después de reparar el CRT estático.
- Al aplicar `PrepareWsl`, el log de Burn dice que el paquete terminó con `code: 0xe` y luego falla con `0x8007000e`.
- `crates/host-preflight/src/exit_codes.rs` define `REBOOT_PENDING = 14`.
- `installer/bundle.wxs` solo mapea `0` a success y `3010` a `scheduleReboot`; el catch-all trata 14 como error.

Resultado requerido:

1. Mapea el exit code real `REBOOT_PENDING` de `PrepareWsl` a `scheduleReboot` en Burn, antes del catch-all. Conserva el mapeo 3010 y el resto de la cadena sin cambios.
2. Añade una comprobación automatizada de contrato al build del instalador para evitar que el valor Rust y el mapeo WiX vuelvan a desincronizarse. La comprobación debe leer el valor de `REBOOT_PENDING` desde `crates/host-preflight/src/exit_codes.rs` y verificar que el `ExePackage Id="PrepareWsl"` contiene ese `ExitCode` con `Behavior="scheduleReboot"`; no dupliques silenciosamente una constante sin validarla contra Rust.
3. Si la sintaxis/namespace XML de WiX vuelve frágil el parseo, usa una validación textual bien acotada al bloque `PrepareWsl`, con mensajes de error claros. No introduzcas dependencias nuevas.
4. Añade o actualiza solamente la documentación técnica necesaria para explicar que Burn programa el reinicio cuando el helper devuelve 14; no conviertas esta prueba Dockur en evidencia G5.

Validaciones obligatorias:

- Parseo de todos los PowerShell modificados.
- `cargo fmt --all -- --check`.
- Pruebas Rust existentes relevantes o `cargo test --workspace` si es viable.
- Build completo `installer/build.ps1` para que WiX valide el bundle y el nuevo guard de contrato.
- Confirma el SHA-256 y tamaño de los nuevos `QuetzalcoatlSetup.exe` y `Quetzalcoatl.msi`.
- `git diff --check`.

Entrega resumen, causa confirmada y resultados. No hagas commit ni push.
