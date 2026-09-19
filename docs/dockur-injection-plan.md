# Dockur/Windows: plan de inyeccion aislada

**Base de revision:** GitHub `main`/`eb692f3` y el digest local fijado
`docker.io/dockurr/windows@sha256:0cff9eb0e7aee9953e55bc682852ca4fdca233145a58ae1ec94f0b0c01a2ed30`.
**Entorno observado:** WSL2 `gnx-node`, Podman 4.9.3, 2026-09-18.
**Estado:** plan ejecutable; no es evidencia de arranque del huesped ni de `READY` GNX.

## Fuentes publicas

- [Dockur Windows README](https://github.com/dockur/windows/blob/master/readme.md): uso,
  requisitos, consola web, `/storage`, `/shared`, `/oem` y `COMMAND`.
- [Variables de entorno](https://github.com/dockur/windows/blob/master/docs/environment.md):
  valores soportados para boot, red, display, web, Samba, instalacion y apagado.
- [Dockerfile fijado por el upstream](https://github.com/dockur/windows/blob/master/Dockerfile):
  volumen `/storage`, puertos `3389`/`8006`, defaults y entrypoint.
- [Entrypoint](https://github.com/dockur/windows/blob/master/src/entry.sh): orden de
  inicializacion y condicion de ejecucion de QEMU.
- [Compose oficial](https://github.com/dockur/windows/blob/master/compose.yml):
  `/dev/kvm`, `/dev/net/tun`, `NET_ADMIN`, puertos y `stop_grace_period`.
- [Codigo fuente y revision publicada](https://github.com/dockur/windows/tree/efe47da76d49c9d77c0a26799c70315fa4d91055):
  referencia del `org.opencontainers.image.revision` observado en la imagen local.

Las paginas anteriores son documentacion publica. No se usaron URLs privadas, tokens,
claves, credenciales ni archivos de actualizacion.

## Hechos observados, sin inferencias

La imagen fijada existe localmente y `podman image inspect` devolvio:

```text
config ID: 35f527c021ffde4ed5b7706bf8927b565ab13b99579a9acbd5ea0c6464ed53b9
digest: sha256:0cff9eb0e7aee9953e55bc682852ca4fdca233145a58ae1ec94f0b0c01a2ed30
architecture/os: amd64/linux
revision label: efe47da76d49c9d77c0a26799c70315fa4d91055
version label: 6.05
entrypoint: /usr/bin/tini -s /run/entry.sh
volume: /storage
exposed: 3389/tcp, 8006/tcp
image env: VERSION=11, RAM_SIZE=4G, CPU_CORES=2, DISK_SIZE=64G
```

El entrypoint carga los módulos de instalación, disco, display, red, Samba, boot y
finalización, y sólo después construye y ejecuta `qemu-system-x86_64`. Por ello un
contenedor `running` o un log de descarga no demuestra que QEMU o Windows hayan iniciado.

La inspección del host fue de lectura únicamente: WSL `gnx-node` estaba `Running`,
systemd `running`, Podman era `4.9.3`, el host era Linux/amd64, `rootless=false` y
`/dev/kvm` era legible. Permanecieron los contenedores GNX preexistentes; no se inspeccionan
ni copian sus entornos, montajes o credenciales en este plan.

## Settings soportados y límites de uso

| Objetivo | Setting upstream | Decisión para la prueba GNX |
|---|---|---|
| Instalación/boot | `VERSION=11`; `BOOT_MODE=windows`, `windows_secure` o `windows_legacy`; `KVM=Y` | Usar `VERSION=11`, `BOOT_MODE=windows`; sólo añadir `windows_secure` si el gate de TPM/SMM lo exige. Pasar `/dev/kvm`. |
| CPU/RAM | `CPU_CORES=2`, `RAM_SIZE=4G` por defecto | Mantener defaults para reproducibilidad; verificar memoria libre antes de iniciar. |
| Disco | `DISK_SIZE=64G`; `DISK_FMT=raw`; `STORAGE=/storage` | Un volumen Podman nuevo y con nombre de prueba, montado sólo en `/storage`. No usar rutas GNX ni `legacy`. |
| Persistencia | `/storage` guarda disco, firmware y descargas; el README permite bind mount o volumen | La persistencia queda limitada al volumen temporal explícitamente creado. Conservarlo sólo si el coordinador autoriza una segunda fase; eliminarlo al terminar el gate. |
| Red/KVM | Compose oficial usa `/dev/kvm`, `/dev/net/tun`, `NET_ADMIN` y publica `8006`/`3389` | En primera fase usar red privada del contenedor y sólo `-p 127.0.0.1:8006:8006`; no usar macvlan, DHCP, passthrough USB/discos ni publicar a LAN. Añadir `/dev/net/tun` sólo si el arranque lo requiere. |
| Inyección de archivos del guest | `/oem` se copia a `C:\OEM`; `install.bat` se ejecuta al final; `COMMAND` agrega un comando al batch | No inyectar secretos ni binarios GNX. Para un fixture inocuo, montar un directorio temporal como `/oem` con un `install.bat` que sólo escriba una marca de prueba; registrar su hash, no su contenido sensible. |
| Compartición guest/host | `/shared` aparece como `Shared` y unidad `Z:`; Samba activo por defecto (`SAMBA=Y`) | No montar `C:\Program Files\GNX`, `C:\ProgramData\GNX`, raíces `legacy` ni `/var/lib/gnx`. Si se prueba la función, usar un directorio temporal vacío y `SAMBA_READONLY=Y` primero. |
| Consola web | `WEB=Y`, `WEB_PORT=8006`, `DISPLAY=web`; el README indica abrir `http://127.0.0.1:8006/` | Sólo loopback. La página HTTP prueba disponibilidad de la interfaz, no que el guest terminó el boot. No habilitar `PROTECT` sin un canal de secreto aprobado. |
| RDP | El upstream expone `3389/tcp` y `3389/udp` | No publicar en la primera fase; sólo probarlo en una fase posterior con ACL y credenciales entregadas fuera de logs. |
| Apagado | `SHUTDOWN=Y`, `TIMEOUT=105`; Compose usa `stop_grace_period: 2m` | Usar `--stop-timeout 120` y retirar el contenedor con objetivo exacto. |

`COMMAND`, `/oem`, `/custom.iso`, `VERSION` como URL y cualquier fichero ISO son
superficies de ejecución o descarga. No deben apuntar a ubicaciones GNX, URLs privadas,
actualizadores ni contenido no revisado. El upstream soporta `/custom.iso`, pero este plan
no lo habilita: primero hace falta un artefacto aprobado y un hash registrado.

## Procedimiento ejecutable, sólo desechable

Ejecutar desde PowerShell y sustituir únicamente el nombre de prueba si hay colisión. Las
órdenes de inspección iniciales no mutan `gnx-node`:

```powershell
wsl.exe --list --verbose
wsl.exe -d gnx-node -- systemctl is-system-running
wsl.exe -d gnx-node -- podman version
wsl.exe -d gnx-node -- podman info --format 'host={{.Host.OS}} arch={{.Host.Arch}} rootless={{.Host.Security.Rootless}}'
wsl.exe -d gnx-node -- test -r /dev/kvm
wsl.exe -d gnx-node -- podman image inspect 'docker.io/dockurr/windows@sha256:0cff9eb0e7aee9953e55bc682852ca4fdca233145a58ae1ec94f0b0c01a2ed30'
```

La fase de ejecución debe usar un volumen y contenedor únicos, `--pull=never`, el digest
completo y sin mounts adicionales:

```bash
podman volume create gnx-dockur-plan-<date>-volume
podman run -d --pull=never --name gnx-dockur-plan-<date> \
  --device /dev/kvm --cap-add NET_ADMIN \
  --env VERSION=11 --env BOOT_MODE=windows \
  --env RAM_SIZE=4G --env CPU_CORES=2 --env DISK_SIZE=64G \
  --env WEB=Y --env WEB_PORT=8006 --env DISPLAY=web \
  --env SAMBA=Y --env SAMBA_READONLY=Y \
  --volume gnx-dockur-plan-<date>-volume:/storage \
  --publish 127.0.0.1:8006:8006 --stop-timeout 120 \
  docker.io/dockurr/windows@sha256:0cff9eb0e7aee9953e55bc682852ca4fdca233145a58ae1ec94f0b0c01a2ed30
```

Si el log indica que el networking de QEMU requiere TUN, detener y documentar el motivo;
no improvisar privilegios. Una segunda ejecución aprobada puede añadir exactamente
`--device /dev/net/tun`, manteniendo el resto de límites.

Durante un timeout acotado, observar sólo estado, procesos y logs sanitizados:

```bash
podman inspect gnx-dockur-plan-<date> --format 'status={{.State.Status}} running={{.State.Running}} exit={{.State.ExitCode}}'
podman top gnx-dockur-plan-<date>
podman logs --tail 200 gnx-dockur-plan-<date>
curl --fail --max-time 5 http://127.0.0.1:8006/
```

No copiar logs que contengan URLs de descarga, identificadores de sesión o credenciales.
Al finalizar, incluso ante fallo, limpiar sólo los objetivos exactos:

```bash
podman rm -f gnx-dockur-plan-<date>
podman volume rm gnx-dockur-plan-<date>-volume
```

Verificar después que los contenedores y volúmenes GNX preexistentes no cambiaron. No usar
`podman system prune`, `wsl --unregister`, reinicios de `gnx-node`, `--privileged`, mounts
amplios ni comandos destructivos con globs.

## Gates de evidencia y readiness

1. **Imagen:** el digest local, arquitectura, entrypoint, labels y defaults coinciden con
   el registro de esta página; si no, detenerse.
2. **Host:** WSL2, systemd, Podman, memoria suficiente y `/dev/kvm` legible; esto prueba
   prerrequisitos, no el guest.
3. **Proceso:** `podman top` muestra `qemu-system-x86_64` y no sólo `aria2c`; sin QEMU,
   el resultado es `BLOCKED` aunque el contenedor siga `running`.
4. **Consola:** `8006` responde en loopback y la interfaz muestra la pantalla del guest.
   HTTP 200 por sí solo no prueba Windows.
5. **Boot guest:** la pantalla muestra escritorio o login de Windows y una acción dentro
   del guest confirma que responde; documentar captura/fecha sin credenciales. Sólo entonces
   se puede decir “guest boot observado”.
6. **Inyección/compartición:** con fixtures no secretos, comprobar `C:\OEM`/`install.bat`
   y `Shared`/`Z:` desde el guest, además del modo sólo lectura del share. Esto no habilita
   GNX ni cambia el contrato de Compute.
7. **GNX READY:** requiere además el health check autenticado y específico de Compute,
   desde el plano GNX, contra el servicio declarado. Ningún estado de Dockur, página web,
   proceso QEMU o escritorio equivale por sí solo a `READY` GNX.

El intento previo documentado en [`dockur-acceptance.md`](dockur-acceptance.md) sólo llegó a
la descarga de ISO y no mostró QEMU; por tanto el estado correcto sigue siendo bloqueado/no
READY hasta que los gates anteriores tengan evidencia independiente.
