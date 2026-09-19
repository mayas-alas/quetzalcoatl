# Dockur/Windows acceptance attempt

**Fecha:** 2026-09-18 (America/Mexico_City)  
**Base:** local `main` at `6e84441`  
**Resultado:** `BLOCKED`; no es `READY` y no se ejecutó ningún instalador ni se modificó el estado legado.

## Alcance y límites

La prueba se hizo contra la distribución WSL existente `gnx-node`, usando Podman
únicamente. No se usaron Docker, montajes de `C:/Program Files/GNX` o
`C:/ProgramData/GNX`, credenciales, claves, URLs privadas, ni archivos de secretos.
La imagen Windows no se inventó ni se sustituyó por otra imagen: sólo se intentó
el nombre público `docker.io/dockurr/windows:latest` con `--pull=never`, porque el
repositorio no contiene un digest confiable para ese artefacto.

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

## Intento acotado

Antes del intento se inspeccionaron las imágenes locales. Sólo estaban presentes
las imágenes de los servicios existentes (`tailscale`, `dockurr/proxmox` y
`caddy`), no `dockurr/windows`. El comando, sin secretos ni montajes del host,
fue:

```bash
podman run --pull=never --rm \
  --name gnx-acceptance-dockurr-windows \
  --device /dev/kvm --cpus=2 --memory=4g --pids-limit=256 \
  --stop-timeout=10 --log-driver=none \
  docker.io/dockurr/windows:latest
```

Salida exacta del bloqueo:

```text
Error: docker.io/dockurr/windows:latest: image not known
```

No se hizo `podman pull`: no había un digest aprobado en el repositorio y la
prueba exige artefactos confiables disponibles localmente. El comando de limpieza
con objetivo exacto fue:

```bash
podman rm -f gnx-acceptance-dockurr-windows
```

La lista posterior confirmó que sólo permanecían los contenedores previamente
existentes `gnx-access`, `gnx-compute`, `gnx-app` y `gnx-entry`; el contenedor
desechable no quedó creado.

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
