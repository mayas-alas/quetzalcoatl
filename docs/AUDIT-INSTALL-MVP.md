# Auditoría de instalación y cadena de confianza — MVP

**Estado:** BLOQUEADO para release; implementación en curso.
**Alcance:** instalación Windows, bootstrap WSL, transacción de setup, empaquetado Linux/Windows y evidencia.

## Hallazgos confirmados

1. **La cadena de autenticidad del release estaba incompleta (P0).** Se cerró el límite de instalación: `gnx-setup` ahora exige manifiesto `sealed`, firma Ed25519 detached (`manifest.sig`) y verifica contra la raíz pública compilada. El flujo sigue bloqueado para release hasta operar una clave de producción gestionada fuera del repositorio y generar SBOM/atestado.
2. **Había un falso positivo de aceptación (P0).** `install-host.ps1` instalaba el CLI y escribía `READY` aunque runtime/WSL, reinicio, `doctor` y `health` fueran gates posteriores.
3. **El estado transaccional es conservador (P1).** Hay lock exclusivo, journal/snapshot atómicos, rechazo de reparse points, reautenticación después de copiar y rollback condicionado por evidencia de ownership.
4. **La aceptación de host no está disponible en este entorno (P0).** No hay Windows/WSL elevado ni Podman Linux operativo desde esta sesión; `cargo clippy` tampoco está instalado para el toolchain activo.
5. **El build de Windows aún no sella un candidate.** `runtime.lock.json` declara `unsealed` y el build genera un manifiesto de hashes sin autenticación criptográfica.

## Cambios aplicados

- `packaging/windows/install-host.ps1`: `READY` fue reemplazado por `ACTION_REQUIRED/HOST_CLI_INSTALLED`; el siguiente paso explícito es provisionar el runtime autenticado y ejecutar gates post-reinicio.
- `packaging/windows/build.ps1`: genera manifiestos deterministas marcados `unsealed` y comprueba que no falte ningún artefacto.
- `packaging/windows/install.ps1`: rechaza manifiestos `unsealed` o firmas detached ausentes antes de ejecutar o importar cualquier runtime.
- `packaging/windows/seal-manifest.ps1`: añade promoción explícita usando la clave privada sólo desde `GNX_RELEASE_PRIVATE_KEY_FILE`; nunca recibe la clave por argv ni la versiona.
- `src/adapter/windows/setup.rs`: verifica firma Ed25519 antes de consumir artefactos y reautentica la copia staged.
- `README.md`: el estado ya no presenta cobertura de repositorio como aceptación de producto; G0–G6 y host acceptance permanecen bloqueados hasta contar con evidencia reproducible.

## Gates ejecutados

- `cargo fmt --all -- --check`: **PASS**.
- `cargo test --locked`: **PASS** (29 tests ejecutados, 1 integración Linux ignorada por requerir root).
- `cargo clippy --locked --all-targets -- -D warnings`: **BLOCKED**, componente `clippy` no instalado.
- Tests PowerShell: **BLOCKED**, `pwsh` no disponible.
- Instalación/reinicio Windows y bootstrap WSL: **BLOCKED**, host no disponible.

## Criterio de salida del MVP

No se declara `READY` hasta que se cumplan todos:

- manifiesto canónico firmado con la clave de producción y verificado contra la raíz pública incorporada al instalador;
- todos los artefactos y rootfs cubiertos por el manifiesto autenticado;
- candidate generado desde checkout limpio, con SBOM y evidencia sanitizada;
- setup Windows + WSL + reboot + `doctor` + `health` ejecutados en host desechable;
- recuperación/rollback y reinstalación probados; un gate fallido conserva `FAILED` o `BLOCKED`;
- G0–G6 verificados desde cliente remoto, no desde loopback/noVNC.

La prioridad es cerrar falsos positivos y mantener el MVP pequeño: no se agregan capas de orquestación al runtime ni se modifica `legacy`.
