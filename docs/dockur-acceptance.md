# Dockur/Windows acceptance attempt

**Fecha:** 2026-09-18 (America/Mexico_City)
**Base:** `99b2dfc` (`docs: record dockur image pull and boot blocker`)
**Resultado:** `BLOCKED`; no es `READY` y no se ejecuto ningun instalador ni se modifico el estado legado.

## Alcance y limites

La prueba se hizo contra la distribucion WSL existente `gnx-node`, usando
Podman unicamente. No se usaron Docker, montajes de `C:/Program Files/GNX` o
`C:/ProgramData/GNX`, credenciales, claves, URLs privadas ni archivos de
secretos. Solo se crearon y eliminaron los recursos desechables con nombres
`gnx-acceptance-dockurr-settings-20260918` y
`gnx-acceptance-dockurr-settings-20260918-volume`.

## Imagen y entrypoint inspeccionados

La imagen fijada fue:

```text
docker.io/dockurr/windows@sha256:0cff9eb0e7aee9953e55bc682852ca4fdca233145a58ae1ec94f0b0c01a2ed30
```

La inspeccion local devolvio config ID
`35f527c021ffde4ed5b7706bf8927b565ab13b99579a9acbd5ea0c6464ed53b9`,
entrypoint `/usr/bin/tini -s /run/entry.sh`, y variables incluidas en la imagen:
`VERSION=11`, `RAM_SIZE=4G`, `CPU_CORES=2`, `DISK_SIZE=64G`.

## Prerrequisitos observados

Comandos ejecutados desde PowerShell:

```powershell
wsl.exe --list --verbose
wsl.exe -d gnx-node -- podman version --format '{{.Client.Version}}'
wsl.exe -d gnx-node -- systemctl is-system-running
wsl.exe -d gnx-node -- test -c /dev/kvm
wsl.exe -d gnx-node -- podman info --format 'host={{.Host.OS}} arch={{.Host.Arch}} rootless={{.Host.Security.Rootless}}'
```

Evidencia sanitizada:

| Gate | Observacion | Estado |
|---|---|---|
| WSL | `gnx-node`, version 2, estado `Running`; kernel `6.18.33.2-microsoft-standard-WSL2` | PASS |
| Podman | `4.9.3`; host Linux/amd64; rootless `false` | PASS |
| systemd | `running` | PASS |
| KVM | `/dev/kvm` presente y legible; modulo `kvm_amd` cargado | PASS parcial |
| virtualizacion CPU | `svm` observado en `/proc/cpuinfo` | PASS parcial |
| Servicios GNX existentes | `gnx-access`, `gnx-compute`, `gnx-app` y `gnx-entry`: `active` | PASS de baseline |

KVM solo quedo comprobado a nivel de dispositivo/modulo; no se puede declarar
la capacidad Compute completa porque el huesped Windows nunca llego a arrancar.

## Retry acotado con settings documentados

Se creo un volumen Podman temporal y se lanzo el contenedor con los settings
requeridos, `/dev/kvm` y `NET_ADMIN`, sin montajes del host:

```bash
podman volume create gnx-acceptance-dockurr-settings-20260918-volume
podman run -d --pull=never \
  --name gnx-acceptance-dockurr-settings-20260918 \
  --device /dev/kvm --cap-add NET_ADMIN \
  --env VERSION=11 --env RAM_SIZE=4G --env CPU_CORES=2 --env DISK_SIZE=64G \
  --volume gnx-acceptance-dockurr-settings-20260918-volume:/storage \
  --stop-timeout=10 \
  docker.io/dockurr/windows@sha256:0cff9eb0e7aee9953e55bc682852ca4fdca233145a58ae1ec94f0b0c01a2ed30
```

La inspeccion del contenedor confirmo:

```text
status=running running=true exit=0
Mounts: solamente el volumen temporal en /storage
Devices: /dev/kvm -> /dev/kvm
CapAdd: [CAP_NET_ADMIN]
Env: VERSION=11, RAM_SIZE=4G, CPU_CORES=2, DISK_SIZE=64G
Entrypoint: /usr/bin/tini -s /run/entry.sh
```

Durante el limite de 60 segundos, `podman inspect` siguio reportando
`status=running`, `running=true`, `exit=0`. Los logs observados fueron:

```text
Starting Windows for Podman v6.05...
Requesting Windows 11 from the Microsoft servers...
Downloading Windows 11...
```

La inspeccion de procesos mostro `aria2c` descargando la ISO y no mostro
QEMU ni un proceso de Windows. Por tanto, el huesped no arranco y no existe
un codigo de salida del guest que pueda interpretarse como exito.

## Limpieza y conclusion

La limpieza tuvo objetivos exactos y retiro ambos recursos desechables:

```bash
podman rm -f gnx-acceptance-dockurr-settings-20260918
podman volume rm gnx-acceptance-dockurr-settings-20260918-volume
```

La lista posterior confirmo que solo permanecian los contenedores existentes
`gnx-access`, `gnx-compute`, `gnx-app` y `gnx-entry`; el volumen temporal ya no
existia. No se verifico servicio Windows invitado, WSL dentro del invitado,
Compute del invitado, Access, Control, bundle completo, consola, conectividad
ni control de extremo a extremo. Por tanto no se afirma `READY`, no se afirma
aceptacion GNX y no se modificaron `gnx-node` ni los directorios legacy fuera
de la imagen, contenedor y volumen desechables de esta prueba.

El bloqueo reproducido queda en la preparacion/descarga de la ISO dentro del
entrypoint de Dockurr antes de iniciar QEMU. Un siguiente intento necesita un
artefacto confiable de `dockurr/windows` (idealmente con digest registrado y
aprobado), manteniendo estos limites y sin montajes de secretos.
