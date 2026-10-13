# Continuity dedicado — ejecución en curso

Decisión operadora: migrar la instancia business existente a un LXC dedicado
en compute.gnx, con Continuity y NetBird BYOP como Quadlets. Reutilizar RustFS,
Pocket ID, management/dashboard NetBird y gateway existentes. Podman 6+ es
requisito. No duplicar infraestructura ni crear controlador por aplicación.

## Preservación y aceptación

- Scope business, prefijo continuity/business y journal existentes se preservan.
- Un único escritor: detener y verificar el anterior antes de activar el nuevo.
- Conservar el spool anterior para recuperación; no borrar objetos S3 ni adoptar
  guests por VMID supuesto. Validar ownership y receipts.
- Verificar head/generación, réplica sin pendientes/errores, lectura y pruebas de
  escritura/restauración sólo en un prefijo de aceptación separado.
- HTTPS mediante infraestructura existente; BYOP account cluster Online,
  management_connected e initial_sync_complete. No exponer puertos públicos.
- Reinicio y nueva comprobación del mismo EXE instalado antes de publicación.

## Retiro gradual AI autorizado

Inventariar guest, ownership, unidades, volúmenes, datos, secretos/referencias y
rutas antes del retiro. La CLI actual no implementa destroy/deprovision: no
sustituirlo por borrado root ad hoc. Detener por lifecycle soportado; sólo retirar
datos/guest identificados tras inventario y recorrido autorizado versionado.
No eliminar RustFS, business, cuentas, CA, identidad WSL ni recursos compartidos.
Reutilizar componentes necesarios para Continuity/BYOP antes de retirar fuente
AI sin consumidores. Registrar reducción neta de líneas por hito.

## Estado comprobado al iniciar

- Instalado gnx 0.1.91; candidato fuente 0.1.92 no entregado.
- GnXHostAgent Running, Automatic; sin acciones SCM de recuperación configuradas.
- compute.gnx HTTPS 200; computer.gnx no resuelve (no es el nombre configurado).
- app.gnx HTTPS 502; checkpoint Blocked/WORKLOAD_MESH_NOT_READY.
- La última inspección AI reporta Podman 5.4.2. El bundle offline existente
  también contiene 5.4.2; no satisface el requisito 6+.

Todo lo anterior describe trabajo pendiente, no una release aceptada.

## Recuperación host e inputs

El candidato MSI declara recuperación SCM mediante util:ServiceConfig:
restart en los tres niveles de fallo, demora 60 segundos y reset de un día.
`node tests/installer/test-uninstall.mjs` pasa. No instalado todavía; el estado
SCM vivo anterior sigue sin acciones. Esto no promete disponibilidad absoluta
ni anula una parada deliberada del servicio.

El índice oficial Debian publica Podman 6.1.1, pero upstream 6.1.3 corrige
CVE-2026-94603. No se ha descargado/installado un paquete 6.x ni cambiado el
bundle sellado. Validar procedencia, dependencias y versión corregida antes de
construir inputs. No usar podman-remote como sustituto del engine local.

La persona operadora excluye app.gnx del esfuerzo actual; mantener observación
honesta de su estado sin gastar trabajo en corregirlo.

## Inventario y contención

`preflight/runtime-inspection.jsonl` observa los archivos de identidad preservados
pero BackendState NoState en ambos sidecars, con errores de permisos del daemon.
El directorio raíz:65532/0770 no cambió; no inferir drift por el formato octal
770 frente a 0770 del primer inspector. RustFS está inactive y Continuity business
reporta REMOTE_UNAVAILABLE/pending. Recuperar esta dependencia antes de migrar.

`gnx workload ai stop --confirm` completó por CLI instalada; guest detenido y
persistencia retenida (`stop-ai.log`). No destroy ni borrado de guest/datos.
El inventario AI observa tres contenedores, cero volúmenes Podman y directorios
de aplicación existentes; esto no demuestra ausencia de datos que conservar.

Builder GnX-LinuxLab: Podman 5.4.2; go y los paquetes libgpgme-dev,
libseccomp-dev/libdevmapper-dev no están instalados. No se realizó una compilación
de Podman 6 ni se alteró el runtime productivo con herramientas de build.

La política raíz:65532/0770 introducida en 0.1.91 se identifica como regresión
de persistencia: upstream Tailscale aplica chmod 0700 a su state dir al iniciar;
el daemon UID65532 no puede hacerlo con propietario root. Corregir en fuente y
empaquetar antes de recuperar RustFS, sin conceder nuevas capacidades ni rekey.
La devolución del socket al recorrido del daemon no acredita TLS nativo AI;
ese trabajo sigue retirado de la prioridad actual.

## Hito instalado 0.1.92: dependencia recuperada

Build canónico pasó, instalador exit 0 y hashes de CLI/agente instalados coinciden
con el build (`host-recovery/built-artifacts.json`, `installed-artifacts.json`).
78 pruebas Rust y suite estática pasan. El fixture del clasificador de certificados
se corrigió para separar sus resultados con saltos de línea; no se relajaron sus
expectativas. La recuperación SCM instalada devuelve tres acciones restart/60s,
reset86400s; GnXHostAgent Running/Automatic. No se provocó un crash artificial.

La primera reconciliación todavía observó NoState del proceso existente.
`gnx workload private-services stop --confirm` y `... start` reiniciaron sólo el
guest compartido por lifecycle instalado, conservando persistencia. El adaptador
reportó healthy. La inspección posterior confirma mesh Running en su mismo nombre
e IP; directorio65532:65532/0700, archivo de estado0600, helper hash instalado.
RustFS/Pocket/NetBird/Continuity activos; business generación0, ready=true,
réplica enabled, pending=false, error=null. Imagen business original preservada.
No escritura productiva ni prueba de durabilidad S3 de nueva generación.

HTTPS con CA/nombre validados (revocación best-effort por CA privada sin CRL):
compute.gnx200, rustfs.gnx/rustfs/console/200, continuity.gnx302.
AI101 permanece stopped; no eliminación de datos ni guest nuevo creado.
Orca creó la pestaña RustFS pero snapshot falló por desconexión de runtime;
no se acredita dashboard/login a partir de esa apertura.

Esto es un hito de recuperación, no aceptación del nuevo LXC Continuity,
Podman6, clusterBYOP ni publicación final. Evidencia en host-recovery/.

## Limpieza y compilación Podman en curso

Se retiraron 212 líneas de lifecycle genérico parcial de la tarea cancelada y
18 de admisión no implementada; se agregaron 9 de regresión: reducción neta221.
El allowlist actual sigue rustfs|ai hasta disponer del lifecycle común probado.
La prueba compila en memoria el Python de deploy.run para detectar errores de
indentación. `node tests/test-workload-deployment.mjs` pasa.

Inputs Podman/Go sellados en `installer/inputs/podman.json`, archivos hidratados
en build/podman/inputs. Tag upstream6.1.3 verificado por la API GitHub de tags,
commit85b994955e0b4e30fbce9c8351cab85676140ede. Go1.26.6 validado contra SHA256
publicado por go.dev. Snapshot incluye vendor; compilación GOPROXY=off,
GOSUMDB=off, GOTOOLCHAIN=local, GOFLAGS=-mod=vendor.

Laboratorio GnX-LinuxLab recibió 71.4MB de paquetes oficiales Debian para
compilar (240MB instalados adicionales). Fuentes/toolchain/cache/staging son
intermedios administrados exclusivamente bajo /var/lib/gnx-build/podman.
Responsable: coordinador; presupuesto inicial de este build: hasta4GiB.
El make upstream compila engine local/rootlessport/Quadlet con seccomp/systemd,
no un cliente podman-remote. Build pendiente, no importado a producción.
Dependencias runtime completas, paqueteoffline, lifecycle, migración, BYOP y
aceptación siguen pendientes. El journal business continúa en su guest actual.

## Engine6 validado en laboratorio

Make upstream completó exit0. Engine y Quadlet reportan6.1.3; engine identifica
Go1.26.6, commit85b994955e0b4e30fbce9c8351cab85676140ede y build1790686827.
`podman info` con storage/runroot aislados confirma cgroupv2/systemd,
conmon2.1.12 y runtime disponible. `ldd` observa libc y libseccomp únicamente
como dependencias directas dinámicas del engine.

Se importó sólo Continuity desde el archiveOCI sellado existente, con config
e476d228157ed0077cddc6bcaa6af686a84492358a4b20472f78c4c9d84e7a26 original;
se ejecutó --help con digestf5cbdbe57d3e34341f00c902db72123d8f6cfd1408d9dc567c729bff735e8884,
UID10001, rednone, readonly, cap-dropall, no-new-privileges y límites128MiB/1CPU.
Exit0, sin spool/secretos de producción. Quadlet6.1.3 dryrun de las definiciones
existentes completó exit0; no equivale a servicioS3 ni clusterBYOP aceptados.
Intermedios del laboratorio:1.1GiB antes del import; storage de prueba separado.
Evidencia: podman-native-build.log, podman-engine-info.log,
podman-continuity-smoke.log y podman-quadlet-dryrun.log.

Se prepararon únicamente las dos definiciones necesarias y su red en
payload/node/workloads/continuity: continuity.container, netbird-proxy.container,
continuity.network. Conservan scopebusiness/prefijocontinuity/business/ACKremote;
reutilizan los digests verificados, sin token inline ni credencialRustFSadmin.
Exigen guardunit antes del arranque; aún no se suministran secretos ni se inicia
el nuevo escritor. Quadlet6.1.3 genera ambas unidades y su networkunit con exit0
(`continuity-quadlet-dryrun.log`); regresiones de definición pasan.
Esto no acredita guest nuevo, admisión, migración, S3 ni clusterOnline.

BuildPodmanInput del build.ps1 existente ya completó con inputs hashvalidados;
se corrigieron el nombreGo y el paso de argumentosPowerShell/WSL en sus pruebas
reales. Reusa caché/source del workspace; sólo recompone staging propio.

La prueba real de red descubrió que Netavark1.x no soporta la APIcreate de
Podman6 (unrecognized subcommand create). Se incorporaron binarios upstream
Netavark/Aardvark2.1.0 verificados contra digests de releases oficiales, sin
presentarlos como compilaciónlocal. Networkcreate aislado y run de Continuity
con redbridge/puerto publicado/UID10001/readonly/cap-dropall completaron exit0.
La red de prueba gnx-build-podman6-smoke se retiró por nombre exacto.

Archive canónico resultante45325783bytes,
SHA256b693b4ae4a44709c8c3d4519525b9d32aa06012531ef201155d8ceac244c2e68;
readback local coincide con manifest. Ver podman-build-manifest.json,
podman-archive-verification.json y podman-network-smoke.log.
El bundle de dependencias/runtime aún no está completo ni integrado/instalado
en el LXC productivo. Servicio Windows comprobado Running/Automatic.
