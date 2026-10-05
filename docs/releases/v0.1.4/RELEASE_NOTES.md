# GnX Node v0.1.4

## Cambios

- Sustituye el comando de host `gnx-setup` por la CLI única `gnx`.
- Expone `gnx status`, `gnx setup`, `gnx retry`, `gnx join`, `gnx cancel`, `gnx help` y `gnx version`.
- Añade `gnx set password` para definir o rotar de forma interactiva la contraseña de Proxmox (`root@pam`) sin pasar secretos por argumentos ni reemplazar la inscripción mesh existente.
- Conserva `gnx password` y `gnx provision` como alias compatibles de la operación segura.
- Actualiza el instalador y el PATH del sistema; una actualización elimina el ejecutable anterior `gnx-setup.exe` y preserva estado, CA, identidad, distro y credenciales existentes.

## Validación realizada

- `cargo test --workspace --locked`
- `node tests/test-static-contracts.mjs`
- `installer/scripts/build.ps1`
- `installer/scripts/verify.ps1`
- Actualización real de v0.1.3 a v0.1.4 en Windows.
- Confirmados `GnXHostAgent` activo, `gnx 0.1.4` disponible mediante PATH, ausencia del ejecutable legado y coincidencia binaria entre el ejecutable construido e instalado.
- Confirmado acceso HTTPS local a `https://compute.gnx` con respuesta HTTP 200.

La contraseña debe introducirla el administrador directamente en la consola elevada mediante `gnx set password`; no se incluye ni se genera una contraseña en esta versión.
