# Propuesta de taxonomía GNX 0.3.1

Status: propuesta puntual, no normativa  
Alcance: instalable GNX 0.3.1, Windows/Wide Linux, runtime Linux y evidencia 360  
Fecha: 2026-09-19

Esta propuesta no cambia el contrato público GNX ni declara READY. Resume inconsistencias reales vistas en `README.md`, docs 01-07, tracker, `packaging/windows`, `packaging/linux` y módulos Rust de setup/runtime/report/ports.

## Diagnóstico

1. La línea normativa ya converge en **Access / Control / Compute**, cuatro operaciones (`doctor`, `plan`, `apply`, `status`) y tres estados públicos (`READY`, `FAILED`, `ACTION_REQUIRED`), pero todavía conviven nombres internos de etapa (`PREFLIGHT`, `PROVISIONED`, `DRAINING`), códigos de setup (`SETUP_*`, `WSL_*`) y vocabulario UI (`RUNNING`) sin un mapa único.
2. `src/main.rs` implementa el contrato nuevo para el binario principal, mientras `src/cli.rs` conserva una superficie previa (`install`, `connect`, `access`, `credentials`) que el tracker aún cita como existente; al no estar exportada desde `lib.rs`, debe tratarse como residuo de código o herramienta histórica hasta que un dueño decida retirarla, conectarla o documentarla fuera del contrato público.
3. Windows tiene dos rutas de instalación con propósitos distintos: `packaging/windows/install.ps1` delega en `gnx-setup.exe` y exige manifest sellado/rootfs verificado; `packaging/windows/install-host.ps1` instala CLI/retira Quetzalcoatl histórico y devuelve `ACTION_REQUIRED HOST_CLI_INSTALLED`. La taxonomía debe evitar llamar a ambas “installer” sin subtipo.
4. El setup Rust protege bien los límites de confianza: manifiesto Ed25519, hash exacto de artefactos/rootfs, staging privado, cuenta `gnx-runtime`, servicio `GNXRuntime`, journal y rechazo de legacy. Quedan gates de host que no se pueden convertir en PASS sin Windows elevado, rootfs verificado, firma real y aceptación de reboot.
5. Hay una tensión de release: `packaging/windows/build.ps1` genera `manifest.json` `unsealed`; `seal-manifest.ps1` muta a `sealed` y firma. Esto es correcto, pero la evidencia debe nombrar el estado como **build candidate unsealed** vs **release candidate sealed**, no “manifest listo”.
6. Wide Linux está bien usado como vocabulario de producto interno, con WSL2 como adaptador. La antigua ruta `provision-gnx-runtime.ps1` que instalaba/exportaba Ubuntu fue retirada: el runtime sólo consume el rootfs autenticado exigido por el setup actual.
7. Residuo/remoción tiene una taxonomía más rica que setup: el checklist distingue `BLOCKED` de `RECOVERY_REQUIRED`, pero `uninstall.ps1` todavía agrupa fallos capturados como `BLOCKED`; la aceptación ya exige reclasificar como `RECOVERY_REQUIRED` cuando hubo mutación.
8. `compute.gnx` sigue siendo la ruta operativa requerida. `app.gnx` se conserva como superficie propia de presentación GNX (HTML/CSS/JS, servida bajo Control y sin convertirse en una cuarta capacidad); el portal histórico se reincorpora como assets embebidos, pero su serving y gate HTTPS aún requieren host evidence.

## Glosario canónico propuesto

| Término | Uso canónico | Evitar / aclarar |
| --- | --- | --- |
| GNX | Producto y contrato público | Quetzalcoatl en interfaces públicas nuevas |
| Access | Identidad privada, transporte autorizado, DNS `.gnx` | “mesh product” como capacidad pública |
| Control | TLS, root pública, rutas HTTPS explícitas | “proxy” como dominio de negocio |
| Compute | Servicio persistente autenticado detrás de Control | “app” genérica si se refiere al required route |
| Wide Linux | Capa Linux aislada usada por Windows | WSL2 como nombre de producto público |
| WSL2 | Adaptador actual de Wide Linux en Windows | Segunda implementación GNX |
| Intent | `gnx.toml`, portátil, no secreto | Manifest/release o paths internos |
| Release | Definición inmutable autenticada | Build local, hash suelto, `target/release` |
| Candidate | Intent validado + release + estado observado en staging | “dist” incompleto |
| Last valid | Último candidate promovido tras verificación live | Archivos generados sin probes |
| Public state | `READY`, `FAILED`, `ACTION_REQUIRED` | Fases de journal como estados públicos |
| Lifecycle phase | `PREFLIGHT`, `STAGING`, `PUBLISHING`, `REGISTERING`, `SECURING`, `PROVISIONED`, `DRAINING`, `UNREGISTERING`, `CLEANING` | Exit code o health state |
| Residual classification | `REMOVED`, `BLOCKED`, `RECOVERY_REQUIRED` | Éxito sólo por contador/script |
| Stable code | `SETUP_*`, `WSL_*`, `RELEASE_*`, `RUNTIME_*`, `OK` | Mensajes nativos o paths como código |
| Artifact | Binario/bundle/rootfs cubierto por manifest | Descarga no pinneada |
| Evidence | Observación sanitizada reproducible | Capturas/logs con secretos o gates maquillados |

## Mapa de dominios y capas

| Capa | Dominio / módulo | Responsabilidad | Nombres permitidos |
| --- | --- | --- | --- |
| Producto | docs 01-07, README | Resultado, BR, gates G0-G6 | GNX, Access, Control, Compute, Wide Linux |
| Aplicación | `src/app::{doctor,plan,apply,status,setup}` | Casos de uso, transacción y reportes | operaciones, public states, stable codes |
| Dominio | `src/domain/*`, `src/report.rs` | Reglas y vocabulario independiente de host | capabilities, secret kinds, report envelope |
| Puertos | `src/port/*` | Interfaces hacia host/runtime/state/release | traits; sin WSL/Podman/Caddy en nombres públicos |
| Adaptadores Linux | `src/adapter/linux.rs`, Caddy/CoreDNS/Podman/systemd | Observación y reconciliación técnica | nombres de herramientas sólo como adapter/evidence |
| Adaptadores Windows | `src/adapter/windows/*`, `gnx-service` | Setup, cuenta, servicio, broker, Wide Linux | `gnx-runtime`, `GNXRuntime`, `GNX-0.3.1` |
| Packaging | `packaging/windows`, `packaging/linux` | Build, sellado, instalación, uninstall | build candidate, sealed candidate, lab helper |
| Runtime assets | `runtime/*` | Configs y unidades elegidas por release | adapter assets, no contrato nuevo |
| Evidencia | tracker, checklist, evidence index | PASS/FAIL/BLOCKED y residuos | gates, observations, sanitized logs |

## Tabla actual → propuesto → motivo → riesgo

| Actual | Propuesto | Motivo | Riesgo |
| --- | --- | --- | --- |
| “installer” para `install.ps1`, `install-host.ps1`, `gnx-setup.exe` | `host-cli installer`, `setup provisioner`, `runtime bootstrap` | Separar retiro histórico, provisioning y health | Bajo; requiere renombrar docs/evidencia, no binarios |
| `PROVISIONED` como hito ambiguo | `ACTION_REQUIRED / SETUP_PROVISIONED / lifecycle PROVISIONED` | Evita READY prematuro | Bajo |
| `PRECHECK` en `SetupState` vs `PREFLIGHT` en journal/docs | Mantener `PREFLIGHT` para lifecycle; usar `PRECHECK` sólo como setup-state interno o migrarlo luego | Reduce doble vocabulario | Medio si hay fixtures que esperan `PRECHECK` |
| WSL / Wide Linux mezclados | Wide Linux en producto; WSL2 sólo adapter/evidencia | Mantiene contrato portable | Bajo |
| `unsealed` manifest | `build candidate (unsealed)` | Evita confundir build con release | Bajo |
| `sealed` manifest | `release candidate (sealed+signed)` | Explicita firma/raíz de confianza | Bajo |
| `src/cli.rs` comandos `install/connect/access/credentials` | `legacy/unwired CLI surface` hasta decisión | No coincide con README/main | Medio; puede ocultar deuda si no se decide |
| `app.gnx` en tracker | `first-party presentation surface` | Conserva la UI HTML/CSS/JS sin crear otra capacidad de negocio | Medio; assets reincorporados, requiere serving y gate HTTPS de UI |
| `BLOCKED` genérico en uninstall | `BLOCKED pre-mutation` vs `RECOVERY_REQUIRED after mutation` | Clasificación segura de residuos | Medio; requiere cambios de script si se automatiza |
| `gnx-node` | builder/lab distro conflict for setup | Docs ya dicen conflicto en host runtime | Bajo |
| `target/release` | local binaries, not installable release | Evita promoción accidental | Bajo |
| `LAB_ONLY` bundle | lab bundle, not release authenticity | Alinea release docs | Bajo |

## Prioridades

### P0 — antes de llamar instalable completo

- Resolver o documentar formalmente la superficie `src/cli.rs`: eliminarla si está muerta, moverla a herramienta histórica/lab, o conectarla sin romper el contrato `doctor|plan|apply|status`.
- Mantener una única ruta Wide Linux/rootfs: la aceptación debe usar el rootfs cubierto por manifest sellado; no se permiten provisiones dinámicas por nombre de distro.
- Ejecutar en Windows elevado desechable: `--check`, `--provision`, reboot, bootstrap, `doctor`, `status`, residuos y uninstall con observaciones independientes.
- Producir manifest `sealed` + `manifest.sig` con raíz pública compilada y rootfs verificado; `runtime.lock.json` `unsealed` sigue bloqueando release.
- Mantener `PROVISIONED` y `SETUP_REBOOT_REQUIRED` como `ACTION_REQUIRED`, nunca READY.

### P1 — alta/media confianza

- Adoptar este glosario en tracker/evidencia para separar state, phase, code, lifecycle classification y artifact status.
- Definir `app.gnx` como superficie de presentación HTML/CSS/JS; no elevarlo a capacidad ni sustituir `compute.gnx`.
- Normalizar nombres de evidencia: `host-preflight`, `setup-provision`, `wide-linux-bootstrap`, `runtime-health`, `residual-removal`.
- Crear una tabla de códigos estables por prefijo (`SETUP_`, `WSL_`, `RELEASE_`, `RUNTIME_`, `BROKER_`) para docs o tests.
- Documentar explícitamente que `install-host.ps1` es transición/retirement y no el instalador de release 0.3.1.

### P2 — seguimiento útil

- Considerar migrar `SetupState.phase = PRECHECK` a `PREFLIGHT` si no rompe compatibilidad.
- Hacer que uninstall distinga automáticamente `RECOVERY_REQUIRED` cuando falló después de mutar.
- Consolidar assets de branding como artefactos de packaging, sin inventar tray app.
- Añadir comprobación estática de que `src/main.rs` y `wire::opcode` siguen exponiendo sólo cuatro operaciones públicas.

## Plan en tres cortes

### Corte 1 — taxonomía y evidencia (pequeño, seguro)

1. Publicar esta propuesta y resumirla en `docs/TEMP-360-TRACKER.md`.
2. Etiquetar gates actuales: Windows/rootfs/firma como `BLOCKED`, no `FAILED` ni `PASS`.
3. Cambiar evidencia futura a campos: `public_state`, `exit`, `stable_code`, `lifecycle_phase`, `residual_classification`, `artifact_status`.

### Corte 2 — saneamiento de instalación

1. Decidir dueño de `src/cli.rs` y scripts host históricos.
2. Alinear la ruta de rootfs Wide Linux con manifest sellado.
3. Añadir fixtures de instalación/recovery que cubran manifest corrupto, rootfs mismatch, reparse, cuenta/servicio existentes y reboot requerido.

### Corte 3 — aceptación 360

1. Construir candidate completo desde checkout limpio.
2. Sellar manifest, registrar hashes/SBOM/licencias y ejecutar `gnx-setup.exe --check`/`--provision` en host Windows declarado.
3. Verificar post-reboot Wide Linux, broker, `doctor`, `status`, `compute.gnx`, secretos, residuos/uninstall y replay de evidencia G0-G6.

## Criterios de aceptación

- El operador ve sólo nombres GNX y operaciones `doctor`, `plan`, `apply`, `status` para runtime.
- Cada resultado separa `state`, `exit`, `code`, lifecycle phase y evidencia observada.
- `READY` sólo aparece tras probes live requeridos; provision/bootstrap/build no lo emiten por sí solos.
- Manifest sellado y firma Ed25519 cubren todos los artefactos instalados/importados, incluido rootfs.
- `gnx-runtime`, `GNXRuntime`, `C:\ProgramData\GNX-0.3.1` y `GNX-0.3.1` se verifican con SID/ACL/path exactos.
- Reboot conserva identidad privada, last-valid y storage; si el host no permite probarlo, el gate queda `BLOCKED`.
- Uninstall/removal sólo declara `REMOVED` tras ausencia independiente y post-reboot; residuos o queries fallidas no son éxito.
- No hay secretos en Git, argv, logs ni evidencia.

## Decisiones NO hacer

- No convertir `app.gnx` en una cuarta capacidad ni en un sustituto de `compute.gnx`; su UI HTML/CSS/JS debe seguir el contrato GNX y tener gate de presentación separado.
- No adoptar `GNX`, `gnx-node` ni raíces `C:\ProgramData\GNX`/`C:\Program Files\GNX` como migración silenciosa.
- No convertir `target/release`, bundle `LAB_ONLY` o manifest `unsealed` en release instalable.
- No borrar residuos reales ni ejecutar instalación destructiva sin host desechable/elevación/autorización.
- No implementar coordinación de agentes dentro de `gnx`, instaladores o runtime.
- No esconder bloqueos de Windows elevado, rootfs o firma como éxito documental.

## Hallazgos que requieren decisión humana

1. ¿`src/cli.rs` debe eliminarse, convertirse en bin separado/lab o mantenerse como deuda fuera del contrato público?
2. Verificar que ningún helper o instalador reintroduzca provisión dinámica por nombre de distro.
3. ¿Se acepta migrar `PRECHECK` a `PREFLIGHT` en estado persistido, o se conserva por compatibilidad?
4. ¿El uninstaller debe emitir `RECOVERY_REQUIRED` nativo después de mutación, o basta con reclasificación en evidencia para 0.3.1?
5. ¿Qué matriz Windows real reemplaza Dockur para G0-G6 final?

## Gates bloqueados observados

- Host Windows elevado/reboot: bloqueado en este entorno; no se ejecutó setup destructivo.
- Rootfs verificado: bloqueado; `runtime.lock.json` y build local no aportan rootfs sellado.
- Firma de release: bloqueado sin clave privada ni sealing de producción; no se simula.
- Aceptación `compute.gnx` remota/TLS/Compute auth: bloqueada sin runtime completo y cliente autorizado.
