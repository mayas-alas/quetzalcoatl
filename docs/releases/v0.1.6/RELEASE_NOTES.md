# GnX Node v0.1.6

## Corrección

- Serializa la carga del perfil y la ejecución WSL bajo la identidad dedicada `gnxnodesvc`.
- Evita la carrera entre keepalive, monitor de estado y provisioning que podía producir `ERROR_ACCESS_DENIED` durante `gnx set password`.
- Clasifica futuros fallos del límite de identidad WSL como `WSL_IDENTITY_FAILED` en lugar del genérico `ACCESS_DENIED`.
- Conserva el aislamiento de la cuenta, distro, inscripción mesh, CA y estado existentes.

## Seguridad

La contraseña introducida en una operación fallida se consumió y eliminó mediante el transporte one-shot; no quedó aplicada ni almacenada en logs o estado. Debe introducirse nuevamente con `gnx set password`.

## Validación

- `cargo test --workspace --locked`
- `node tests/test-static-contracts.mjs`
- `installer/scripts/build.ps1`
- `installer/scripts/verify.ps1`
- Actualización real del host a v0.1.6 con `GnXHostAgent` activo.
