# GnX Node v0.1.5

## Corrección

- Evita que una desconexión o cancelación de una CLI termine el único hilo del Named Pipe mientras Windows continúa mostrando `GnXHostAgent` como `Running`.
- Los errores de escritura o `FlushFileBuffers` quedan aislados a la conexión que falló; el servicio continúa aceptando `gnx status`, `gnx set password` y las demás operaciones.
- Mantiene la CLI unificada `gnx` y la rotación segura de contraseña de Proxmox introducidas en v0.1.4.

## Causa confirmada

Un cliente que enviaba una solicitud válida y cerraba la conexión antes de consumir la respuesta provocaba un error de broken pipe. Ese error escapaba del bucle servidor y terminaba solamente su hilo. Los hilos de keepalive/estado y el proceso del servicio seguían vivos, por lo que SCM informaba incorrectamente que el servicio continuaba operativo.

## Validación realizada

- `cargo test --workspace --locked`
- `node tests/test-static-contracts.mjs`
- `installer/scripts/build.ps1`
- `installer/scripts/verify.ps1`
- Actualización real del host a v0.1.5.
- Reproducción deliberada de un cliente que envía `GetProgress` y cierra sin leer.
- Cinco solicitudes consecutivas posteriores respondieron correctamente por el pipe.
- `gnx set password` alcanzó correctamente el control de consola segura.
- `https://compute.gnx` respondió HTTP 200 después de la prueba.
