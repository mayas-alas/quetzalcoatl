# Inspección de servicios AI y Continuity

Estado: **inspección y cliente BYOP parcial con pruebas aisladas, no feature desplegado**.
La release instalada sigue siendo v0.1.72. No se construyó ni publicó un nuevo
installer, no se aplicó un hotfix y no se creó/modificó el guest AI en esta inspección.

## Corrección del formato y del alcance de implementación

La persona operadora rechazó los nuevos archivos Python, las plantillas `.in`
y el scaffolding paralelo al ciclo de vida existente. Se detuvieron los workers
y se retiraron esos prototipos, incluido el controlador AI Python y la integración
Rust/IPC incompleta que dependía de él. No se alteró el checkout dirty de Voice.

La definición queda en `payload/node/workloads/ai/profile.json` y seis Quadlets
reales `.container`, con referencias OCI por digest y sin placeholders. El perfil
identifica un LXC no privilegiado, almacenamiento AI independiente, la identidad
Tailscale `gnx-ai` con estado propio y el gateway privado. El empaquetador
`installer/scripts/package-ai.run` es shell; comprueba los Quadlets existentes,
no genera configuración desde plantillas ni cambia el código del componente.

Validación de este bloque: `node tests/test-ai-profile.mjs` (13 contratos de
**definición**, no aceptación runtime), suite estática existente y sintaxis Bash.
El generador Quadlet también se utiliza en modo `--dryrun`, sin arrancar servicios.
La integración debe añadirse al adaptador LXC y al enrolamiento existentes; no hay
un segundo controlador. Registro de manifiestos en S3/Continuity, OAuth por app,
TLS/DNS/Caddy, enrolamiento AI y aceptación real siguen pendientes. Estos archivos
no prueban que exista un LXC AI ni que se haya generado una release nueva. Voice
permanece explícitamente deshabilitado para inferencia hasta disponer de sus
credenciales; una API token interna no habilita audio ni acredita billing.

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

## Inspección de prerequisitos y cliente BYOP

Inspector versionado: `tests/node-runtime/inspect-ai.ps1` + `inspect-ai.run`.
Usa la identidad dedicada existente con su credencial DPAPI sólo en memoria y
transfiere el programa fijo de sólo lectura por stdin. No coloca runners en
Windows Temp ni cambia payload/servicios/credenciales en vivo. Descubre el guest
compartido por tag de ownership y rechaza ausencia/duplicados, sin calcular VMID.

Comando ejecutado:

```text
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/node-runtime/inspect-ai.ps1
```

Informe sanitizado durable: [runtime-inspection.jsonl](runtime-inspection.jsonl).
Observado:

- Sólo guest `100`, compartido; no guest AI creado.
- Imagen Continuity del almacén guest: digest
  `sha256:1afd94a9bccb26985160f3557ce4477f246247f2d93619d3d0add11298e0cd56`.
  Todavía no es prueba de exportación/importación del bundle nuevo.
- Health Continuity: `scope=business`, `ready=true`, generación `0`,
  proyección ready y réplica enabled/sin pending/error. Con generación cero esto
  **no prueba escritura remota ni permisos/restore de un nuevo prefijo S3**.
- Management no requiere nuevo setup. La credencial existente permite consultar
  users y las APIs `reverse-proxies/clusters` y `reverse-proxies/proxy-tokens`;
  ambas devuelven HTTP 200 y count 0. No se expusieron valores ni se mintió un
  token. Lectura autenticada no equivale a aceptación de creación/registro.

Se implementó `payload/node/workloads/ai/proxy-enroll.run` siguiendo la API
v0.80.0 de tokens de cuenta y custom domains. No usa el token global del management,
no adopta registros sin ownership y falla ante cluster shared/offline, dominio sin
verificar, token revocado o plaintext perdido tras interrupción. No se ha integrado
con el provisioner/installer ni se ha activado en el runtime.

Pruebas en memoria, sin red ni archivos temporales:

```text
PYTHONDONTWRITEBYTECODE=1 python3 tests/test-ai-proxy-enrollment.py
node tests/test-static-contracts.mjs
```

Resultado: **13 pruebas BYOP pasan**; contratos estáticos y lifecycle in-memory
existentes también pasan. No son prueba de TLS, DNS CNAME real, cuenta/cluster
conectado, OAuth de aplicaciones o provisioning AI.

Guías leídas: Custom Domains (actualizada 2026-09-12) y Bring Your Own Proxy
(actualizada 2026-09-30), más schemas/handlers/validator de NetBird v0.80.0.
Caddy sigue siendo la entrada TLS; el proxy oficial hace el registro account-scoped.
La selección de apex/custom domains públicos o privados está pendiente de la
persona operadora. No se sustituyó ese dato por un dominio ficticio.

## Trabajo aún no acreditado

Creación idempotente del LXC AI, empaquetado OCI por digest, credencial S3
limitada a `continuity/ai`, Quadlets de AI, integración Voice/Continuity,
HTTPS/OAuth por aplicación, inferencia/audio real, BYOP conectado, aceptación
S3 commit/retry/restore y publicación/descarga del nuevo release en S3.

El alcance y los límites de separación están en
[Máquinas dedicadas y publicación de servicios](../../machine-provisioning.md).
