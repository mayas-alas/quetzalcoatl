# Inspección de servicios AI y Continuity

Estado: **ampliación de alcance e inspección, no feature desplegado**.
La release instalada sigue siendo v0.1.72. No se construyó ni publicó un nuevo
installer, no se aplicó un hotfix y no se creó/modificó el guest AI en esta inspección.

## Fuente Continuity confirmada

La persona operadora indicó `C:\Users\mayas\continuity`; es exactamente el
repositorio inspeccionado, no `continuity-local-backup-*` ni la rama `legacy`.

- Commit observado: `d5c8886` — Implement Rust business continuity MVP with verified S3 durability.
- `git status --short`: sin cambios en ese repositorio.
- Contratos leídos: `README.md`, `docs/contracts.md`, `docs/operations.md`,
  `deploy/README.md`, `deploy/continuity.container` y el handler HTTP del MVP.
- El scope es fijo por instancia; no hay selección de scope por petición ni
  servidor MCP implementado por Continuity.
- El Quadlet business de referencia usa `localhost/continuity:0.2.0`. Ese tag
  **no es evidencia de digest**, de exportación/importación offline ni de que
  el contenedor activo tenga exactamente el código de ese commit. El nuevo
  paquete debe verificar la imagen antes de fijarla por digest.
- La configuración business apunta a bucket `continuity`, prefijo
  `continuity/business` y spool `/var/lib/continuity-business`.
  Es configuración inspeccionada, no un nuevo resultado de lectura/escritura S3.

## Runtime existente: diagnóstico nativo de sólo lectura

Se solicitó el diagnóstico mediante la CLI instalada (`gnx diagnose compute`),
sin crear runners de trabajo. El agente conserva el informe canónico local en
`C:\ProgramData\GnX\Node\state\compute-diagnostic.txt`.

El informe actualizado observado el 2026-10-06 incluye:

```text
--- access-backend-services (exit=exit code: 0) ---
continuity-business Up 2 hours
```

Pocket ID, NetBird server y dashboard también figuran activos/healthy. El guest
existente sigue `running`, hostname `rustfs`, tags `gnx;gnx-rustfs`. Los tres
backends `rustfs.mesh.gnx`, `pocket.mesh.gnx` y `netbird.mesh.gnx` siguen apuntando
a `100.127.95.60`; esta inspección no migró identidad ni DNS.

El snapshot de progreso observado indica `NODE_READY`, fase `ready`, mensaje
`Compute diagnostic captured locally.`. Ese resultado no acredita salud S3 de
Continuity ni aceptación del nuevo stack AI. `Up` tampoco sustituye la consulta
`/health/ready` y el estado de su réplica.

## Browser skill: NetBird existente

Inspección con la guía versionada de `orca computer`, Firefox y ventana
redescubierta `263254`. Se seleccionó la pestaña NetBird y se leyó el árbol de
accesibilidad sin recoger valores de credenciales ni callbacks OAuth.

Origen y pantalla observados:

```text
https://netbird.gnx/reverse-proxy/clusters
Clusters
Proxy clusters route inbound traffic to your services.
Shared clusters are run by the platform; account clusters (self-hosted)
run on your own infrastructure.
No clusters available
Set up a cluster to route traffic through your own infrastructure.
Setup Self-Hosted Cluster
```

No se pulsó el botón de setup ni se generó un token. La evidencia confirma la
pantalla del management existente y ausencia de clusters listados para la cuenta
actual, **no un cluster BYOP conectado ni un proxy desplegado**.

## Pruebas del componente

Comando ejecutado contra la fuente indicada, sin editarla:

```text
cargo test --manifest-path C:/Users/mayas/continuity/Cargo.toml --locked
```

Resultado: **32 passed, 0 failed, 1 ignored**.

- 2 unit tests de almacenamiento/firma.
- 22 tests business: scope fijo, replay, idempotencia, aislamiento de escritor,
  integridad, confirmación explícita, redacción y durabilidad, entre otros.
- 2 tests HTTP/CLI: contratos, configuración por entorno y restart de procesos.
- 6 tests de policy/representación/barriers.
- `conditional_writes_commit_and_restore_against_real_s3`: **ignored**, exige
  endpoint y entorno de credenciales explícitos. Los tests de réplica con
  adapters de pruebas no prueban el RustFS actualmente desplegado.

Las pruebas generan sus salidas normales bajo el `target/` del componente y sus
fixtures propios; no se añadieron scripts temporales ni se alteraron datos
productivos. No se reutilizaron ni imprimieron credenciales S3.

## Trabajo aún no acreditado

Creación idempotente del LXC AI, empaquetado OCI por digest, credencial S3
limitada a `continuity/ai`, Quadlets de AI, integración Voice/Continuity,
HTTPS/OAuth por aplicación, inferencia/audio real, BYOP conectado, aceptación
S3 commit/retry/restore y publicación/descarga del nuevo release en S3.

El alcance y los límites de separación están en
[Máquinas dedicadas y publicación de servicios](../../machine-provisioning.md).
