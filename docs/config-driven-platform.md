# Plataforma config-driven: propuesta de núcleo pequeño y despliegue genérico

Fecha: 2026-10-07. **Diseño aprobado como objetivo; implementación pendiente.**
Normas obligatorias: [contrato de agentes](../.AGENTS/40-config-driven-deployments.md).
Este cambio de dirección no acredita una release AI aceptada ni habilita hoy
upload privilegiado desde la PWA.

## 1. Por qué estamos retrabajando

La plataforma tiene las piezas (LXC, Podman/Quadlet, sidecar, IPC, CLI, CA, S3),
pero el caso AI añadió rutas especializadas en lugar de consumir un catálogo.
Cada aplicación amplió allowlists, comandos, estados, enrolamiento, DNS y auth.
Resultado: preparar un guest no implica que el recorrido completo termine.

Hallazgos de fuente y evidencia:

- `compute/lxc.run` permite sólo `rustfs|ai` y contiene el lifecycle AI específico.
- CLI/IPC/agente tienen lógica AI específica y `JoinWorkload`; hoy hay que pedir
  una clave de mesh aparte. La clave de onboarding original se consume/elimina.
- `workloads/ai/proxy-enroll.run` (194 líneas) y `enroll.run` (366 líneas) contienen
  dos implementaciones de enrolamiento BYOP. Las pruebas del primero no acreditan
  automáticamente el segundo que invoca el adaptador real.
- `workload_statuses()` oculta errores de observación convirtiéndolos a
  `off/available`. Esto debe convertirse en estado desconocido/error explícito.
- Durante el diagnóstico instalado se observó temporalmente CLI no disponible
  aunque SCM seguía Running; luego se recuperó. El informe conserva sondas de
  transporte/STUN fallidas pese a NodeReady. Separar disponibilidad del pipe,
  ejecución de jobs y salud efectiva; informe capturado no significa checks pass.
- El frontend actual muestra servicios conocidos y no importa un bundle Quadlet.
- El candidato v0.1.78 se instaló, pero la evidencia AI no acredita servicios,
  mesh, OAuth y uso real. Hay correcciones de fuente posteriores al EXE.
- Build/publicación no tienen un presupuesto/cierre automático común de espacio.
  El builder acumula almacenes aislados de import además del almacén Podman normal.
- `publish-candidate.py` publica en HF, no en S3. Su texto de notas afirma instalación
  y upgrade sin que ese helper los compruebe: notas deben consumir evidencia real,
  no producir una aceptación por plantilla.

## 2. Qué es "fat" hoy (baseline transparente)

Medido en el working tree actual, no en una release limpia. Líneas físicas con
comentarios, blancos y tests; no métricas de complejidad ni justificación para
borrar checks de seguridad.

| Bloque | Archivos de texto medidos | Líneas |
| --- | ---: | ---: |
| `apps/host-agent/src` (Rust) | 11 | 6,924 |
| `apps/setup/src` (Rust) | 2 | 1,058 |
| `crates/control-protocol/src` (Rust) | 1 | 652 |
| Total Rust de esos bloques | 14 | **8,634** |
| `apps/web-app` (JS/HTML/CSS) | 4 | 298 |
| `payload/node` (scripts/config/Quadlets y contratos) | 72 | 4,531 |

Archivos grandes: `dedicated_account.rs` 1,530; `linux_node.rs` 1,397;
`provisioning.rs` 897; `github.rs` 805; Setup `main.rs` 745;
`control-protocol/lib.rs` 652; `secret_store.rs` 628; `state.rs` 521.
`compute/lxc.run` 447; enrolamiento/instalación AI 366/105.

Clasificación propuesta:

| Mantener en núcleo | Generalizar/eliminar duplicación | Revisar por uso real |
| --- | --- | --- |
| Identidad Windows, DPAPI, DACL, Named Pipe, replay, elevación | Branches por workload/app, listas de servicios y hostname/paths | Integración GitHub nativa heredada frente a flujo web directo |
| Ownership LXC, conservación de discos y CA | Dos enroladores BYOP, health/status específicos | Tablas/tests que cubren definición pero no el lifecycle instalado |
| Transporte y errores sanitizados | Estado/logs divergentes entre CLI y frontend | Copias de OCI y cachés de import/build sin cierre |

**Adelgazar no significa usar Svelte obligatoriamente.** Aquí se interpreta como
reducir acoplamiento/duplicación del producto. Una migración al framework Svelte,
si se desea, es un cambio distinto; el frontend tiene sólo 298 líneas medidas y
no es la causa principal del retrabajo de provisioning.

Métricas de aceptación del refactor: nuevo perfil de prueba sin diff Rust/CLI ni
branch de app; un solo enrolador por integración; pruebas del adaptador realmente
instalado; tamaño/pico de build; tiempo de reconcile; cero temporales del agente
heredados. Reportar diff de líneas y responsabilidades, no prometer un porcentaje
arbitrario de reducción.

## 3. Contratos externos, un único motor

Un bundle lógico contiene estructura fija y manifiesto versionado:

```text
profile.json                 # ID, schema, template/hash, recursos, desired state
images.json                  # digests OCI, archives/hashes/sizes
quadlets/*.container         # Quadlets concretos, no plantillas .in
quadlets/*.network
config/<service>/...          # configuración externa sin secretos
policy/...                   # red/acceso dentro de capacidades aprobadas
checks.json                  # verificadores tipados, no shell libre
manifest.json                # fuente, revisión, hashes, límites y requisitos
```

Paths del ejemplo describen el contrato objetivo, no carpetas ya soportadas.
El loader traduce destinos a `/opt/gnx/workloads/<id>/`, Quadlets a
`/etc/containers/systemd/`, datos a `/var/lib/gnx/<id>/` y secrets a un almacén
protegido. El usuario no decide paths absolutos root ni monta el disco del host.

- Validar archive sin traversal, links, devices, hardlinks ni expansión desmedida;
  limitar archivos, bytes comprimidos/descomprimidos y cantidad de unidades.
- Admitir sólo claves Quadlet/referencias systemd soportadas. Bloquear `Exec*`,
  `PodmanArgs`/opciones de escape, `Privileged`, devices/mounts host no aprobados,
  units `.service` arbitrarias y secrets inline. Dry-run del generador no es sandbox.
- Imágenes por digest y `Pull=never` para bundle offline. Validar el digest importado,
  no asumir que el digest de un tag o almacén dev coincide con el archive.
- Declarar redes, UID, capacidades, volúmenes, CPU/RAM/disco y límites de logging.
  Un servicio que requiera algo nuevo pide una capacidad genérica revisada.
- Secret references por ID autorizado, con ACL/ownership; no contenido secreto en
  uploads, payload público ni receipt. Separar identidad de publicación de consumo.
- Checks tipados: unidad activa, estado mesh, DNS esperado, TLS verificado,
  HTTP readiness/esquema JSON, auth negativa y commit/readback cuando corresponda.
  Eliminar checks por shell proporcionado desde el frontend.
- Dependencias Pocket/NetBird/S3 se registran mediante adaptadores reutilizables
  con intents/receipts, recuperación segura y drift checks. No hacer un script de
  bootstrap independiente por aplicación ni adoptar recursos ajenos.

Registro persistente por máquina: ID lógico, VMID, owner, digest de template,
config deseada/aplicada, recursos, secretos referenciados, identidad mesh,
estado deseado/observado, último error y referencias de logs/evidencia. VMID lo
asigna Proxmox; conservar alias `rustfs` y su propiedad hasta una migración probada.

## 4. Lifecycle instalado

```text
validate → plan → ownership → offline prepare → LXC provision
         → independent mesh enrollment → dependencies/auth
         → apply Quadlets → service checks → access checks → applied receipt
```

Un lock por máquina y receipts por etapa permiten reanudar sin crear otro guest,
rotar secretos, rehacer imports idénticos ni repetir escrituras remotas ambiguas.
Un fallo conserva diagnóstico y ownership; no escribe un receipt de aceptación.
No rollback destructivo de volúmenes/datos; mantener última config validada cuando
sea seguro y mostrar intervención requerida cuando no lo sea.

- Alta: capturar una vez la clave Tailscale reusable con eco desactivado, protegerla
  en secret store y usar staging file para cada peer nuevo. Identidades separadas.
- Upgrade: conservar peers inscritos; enrolar únicamente nuevos sidecars usando
  la misma clave vigente. Key ausente/expirada/revocada es bloqueo explícito.
- Migración actual: si la clave fue borrada, recaptura interactiva una sola vez;
  nunca intentar extraer una auth key de `tailscaled.state`.
- Boot/restart: verificar todas las máquinas cuyo desired state sea running;
  hashes/ownership, Quadlets, mesh, DNS/TLS/auth/health. Un guest deliberadamente
  detenido no arranca por una lectura ni por un reconciliador global.
- Operaciones largas/diagnósticos: jobs acotados y correlacionados; mantener
  lectura de estado disponible sin esperar toda la ejecución. Diferenciar timeout,
  pipe ocupado, servicio ausente y health fallida; no pedir reparar el servicio
  indiscriminadamente ni traducir una captura de diagnóstico a readiness.
- Logs: `journalctl`/fuente canónica, sanitización, tamaño/tiempo/cursor limitados.
  CLI y PWA consumen la misma proyección, con IDs de operación correlacionados.
  Estado del nodo y aceptación de cada workload se presentan por separado.

Sintaxis CLI se define en un contrato futuro común para validate/plan/apply/status/
logs/reconcile; no anunciar comandos no implementados. El CLI actual conserva
onboarding y control elevado por Named Pipe.

## 5. Upload desde frontend sin introducir un shell root

Objetivo de la persona operadora: seleccionar una estructura Quadlet, crear un
nuevo LXC programáticamente y observar estado/logs sin cambios de código base.
La sesión Pocket de la PWA **no equivale a Administrators de Windows**.

Ruta propuesta, para conservar el boundary actual sin otro HTTP localhost:

1. PWA importa/valida el bundle sin ejecutar contenido y presenta plan/hash.
2. Deposita un draft inmutable en el S3 existente con permiso específico de
   upload, TTL/cuotas y origen aprobado. Falta diseñar/autorizar la emisión de
   ese permiso; no poner una clave S3 admin ni Tailscale en JavaScript.
3. Envía sólo un ID de bundle/revisión mediante protocolo nativo instalado.
   El CLI elevado verifica token Windows, origen configurado, hash/manifiesto,
   propiedad y plan; muestra confirmación explícita antes de aplicar.
4. El agente obtiene el bundle sólo del destino permitido y aplica el mismo
   lifecycle que la CLI. No aceptar URL/path/comando elegido por browser.
5. PWA ve progreso/logs sanitizados publicados en la proyección existente.

La primera entrega puede limitarse a importación CLI de bundle validado; después
se añade UI/staging con los mismos contratos. Si se desea despliegue remoto sin
confirmación Windows, exige una decisión nueva de roles, servicio de admisión,
firmas/replay y auditoría; no se implementa implícitamente como este upload.

## 6. Build, offline, nube y limpieza

- Cambiar capacidad genérica del núcleo → nuevo Burn → instalar → aplicar por
  CLI instalada → probar restart/browser → publicar. Nunca completar con hotfix.
- Después de instalar ese motor, una nueva app dentro del schema se entrega como
  bundle config/OCI firmado o aprobado; no requiere recompilar Rust. Su revisión
  aplicada y aceptación quedan registradas igual que la versión del motor.
- HF actual: Space `gnx-labs-mx/quetzalcoatl` para fuente/notas; dataset
  `gnx-labs-mx/gnx-node-releases` para inputs/Burn cuando la cuota lo requiera.
  GitHub/GHCR upstream compute es un destino distinto, no nuestro remote fuente.
- S3 adicional: definir bucket/prefijos de artefactos e inputs offline, CA,
  escritor de publicación y política de retención. El bucket `continuity` visto
  en browser no autoriza usar el prefijo business/AI como repositorio de releases.
- Manifiestos relacionan snapshot/commit fuente, schema/motor, digests OCI,
  archivos/hash/tamaño, revisión aplicada y evidencia. Verificar readback desde
  HF y S3; S3 es almacenamiento de blobs, no un registry OCI por nombrarlo así.
- Reutilizar un archive por digest; límites de caché y build previstos antes de
  importar. Un almacén aislado de aceptación tiene owner y teardown propio.
  No desactivar tests para ahorrar espacio ni borrar caches del runtime productivo.
- El job de build debe cerrar en éxito/fallo con limpieza selectiva, dejar sólo
  candidato vigente/último validado, inputs reutilizables y evidencia necesaria.
  WiX, imports, downloads y compilación cuentan en el pico, no sólo el EXE final.

## 7. Entregas acotadas y gates

| Fase | Entrega | Gate para cerrar |
| --- | --- | --- |
| 0 | Reglas de agentes, inventario, builder limpio | Evidencia durable; lab independiente; runtime intacto |
| 1 | Registro/schema y admisión CLI genéricos | Perfil nuevo aceptado sin branches Rust/app; negativos de escape |
| 2 | Reconciliación LXC/Quadlets/mesh reutilizable | Alta + upgrade + restart; key reusable protegida; stopped respetado |
| 3 | AI como config del motor | BYOP/OAuth/TLS/S3 e integración real; no runners laterales |
| 4 | Frontend upload + logs | Autorización Windows/roles, staging aprobado, límites/replay; browser evidence |
| 5 | Release y inputs en HF/S3, cleanup automático | Download/hash y reproducción; presupuesto/retención de espacio |

No hacer todas las fases como una reescritura big-bang. Mantener ownership de
servicios privados, auth aprobada y source dirty de otros componentes. Cada fase
entrega una capacidad verificable y registra el gate que sigue abierto.
