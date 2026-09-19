# Dockur/Windows acceptance attempt

**Fecha:** 2026-09-18 (America/Mexico_City)  
**Base:** local `main` at `6e84441`  
**Resultado:** `BLOCKED`; no es `READY` y no se ejecutó ningún instalador ni se modificó el estado legado.

## Alcance y límites

La prueba se hizo contra la distribución WSL existente `gnx-node`, usando Podman
únicamente. No se usaron Docker, montajes de `C:/Program Files/GNX` o
`C:/ProgramData/GNX`, credenciales, claves, URLs privadas, ni archivos de secretos.
La imagen Windows se descargó explícitamente con Podman desde
`docker.io/dockurr/windows:latest`; no se usó Docker ni se montaron secretos. La
referencia de plataforma resuelta por Podman fue
`docker.io/dockurr/windows@sha256:0cff9eb0e7aee9953e55bc682852ca4fdca233145a58ae1ec94f0b0c01a2ed30`
(config local `sha256:35f527c021ffde4ed5b7706bf8927b565ab13b99579a9acbd5ea0c6464ed53b9`).

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

| Gate | Observación | Estado |
|---|---|---|
| WSL | `gnx-node`, versión 2, estado `Running`; kernel `6.18.33.2-microsoft-standard-WSL2` | PASS |
| Podman | `4.9.3`; host Linux/amd64; rootless `false` | PASS |
| systemd | `running` | PASS |
| KVM | `/dev/kvm` presente y legible; módulo `kvm_amd` cargado | PASS parcial |
| virtualización CPU | `svm` observado en `/proc/cpuinfo` | PASS parcial |
| Servicios GNX existentes | `gnx-access`, `gnx-compute`, `gnx-app` y `gnx-entry`: `active` | PASS de baseline |

KVM sólo quedó comprobado a nivel de dispositivo/módulo; no se puede declarar la
capacidad Compute completa para esta aceptación porque el huésped Windows nunca
llegó a arrancar.

## Pull e intento acotado

El pull se hizo con el motor existente:

```bash
podman pull docker.io/dockurr/windows:latest
```

Terminó correctamente y devolvió el config digest
`35f527c021ffde4ed5b7706bf8927b565ab13b99579a9acbd5ea0c6464ed53b9`; la
inspección de la imagen devolvió el digest de plataforma anterior. El contenedor,
sin secretos ni montajes del host, se lanzó con:

```bash
podman run --pull=never --rm \
  --name gnx-acceptance-dockurr-windows \
  --device /dev/kvm --cpus=2 --memory=4g --pids-limit=256 \
  --stop-timeout=10 --log-driver=none \
  docker.io/dockurr/windows@sha256:0cff9eb0e7aee9953e55bc682852ca4fdca233145a58ae1ec94f0b0c01a2ed30
```

Durante aproximadamente 90 segundos el contenedor quedó `running`, pero sólo
ejecutó el descargador ISO de Windows (`aria2c`); no apareció un proceso QEMU ni
un huésped Windows arrancado. El evento Podman registró exactamente:

```text
died exit=16
remove exit=0
```

Éste es el bloqueo finito de esta ejecución: Dockurr terminó con código 16 durante
la preparación/descarga de su ISO y no llegó a iniciar el huésped. No se afirma que
el bundle GNX esté presente o completo dentro de la imagen. El comando de limpieza
con objetivo exacto fue:

```bash
podman rm -f gnx-acceptance-dockurr-windows
```

La lista posterior confirmó que sólo permanecían los contenedores previamente
existentes `gnx-access`, `gnx-compute`, `gnx-app` y `gnx-entry`; el contenedor y su
volumen anónimo temporal fueron retirados por `--rm`.

## Gates y conclusión

No se pudo verificar servicio Windows invitado, WSL dentro del invitado, Compute
del invitado, Access, Control, bundle completo, consola, conectividad ni control
de extremo a extremo. Por tanto no se afirma `READY`, no se afirma aceptación
GNX y no se ejecutaron pruebas destructivas o de recuperación.

Para reintentar hace falta proporcionar/importar primero un artefacto confiable de
`dockurr/windows` (idealmente un digest registrado y aprobado en el repositorio),
manteniendo el mismo límite de recursos y sin montajes de secretos. La limpieza
realizada sólo apuntó al nombre desechable de esta prueba; `gnx-node` y los
directorios legacy quedaron intactos.
