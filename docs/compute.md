# Primer servicio de cómputo

`https://proxmox.mesh.gnx` abre Proxmox VE mediante el Quadlet `gnx-compute`
en WSL. La entrada GNX usa 443 y valida TLS también hacia `gnx-compute:8006`;
el contenedor no publica puertos al host. Resolución local por `hosts`.

## Preparar

Requiere el control plane operativo, KVM, FUSE, TUN y 32 GiB libres en la unidad
Windows. El operador autorizó el contenedor privilegiado dentro del mismo WSL.

```powershell
cargo build --release --locked --manifest-path ops/compute/Cargo.toml
# PowerShell elevado:
.\ops\compute\prepare-host.ps1
```

La preparación genera o recupera la credencial, inicia el servicio, configura
HTTPS y comprueba login API antes y después de reiniciar el servicio. También
comprueba que el cliente siga conectado al control plane.

## Configuración y acceso

| Elemento | Ubicación o valor |
|---|---|
| Endpoint, nodo y cuenta | `runtime/compute/compute.toml` |
| Imagen y límites | `runtime/compute/gnx-compute.container`; 3 GiB, 2 CPU |
| Entrada HTTPS | `runtime/compute/compute.caddy` |
| Datos persistentes | WSL `/var/lib/gnx/compute/config` y `storage` |
| Credencial Windows | `%LOCALAPPDATA%/GNX/compute/owner.credential.xml`, DPAPI y ACL |
| Credencial de arranque | WSL `/var/lib/gnx/compute/root.password`, acceso root |
| Resultado sin secretos | `%ProgramData%/GNX/compute-status.json` |

Cuenta `root@pam`: en la interfaz, usuario `root` y realm Linux PAM.
La contraseña procede del archivo protegido; no se documenta su valor.
Windows elimina su copia temporal al terminar la preparación. El arranque
aplica la contraseña del archivo WSL; cambiarla sólo en la interfaz no persiste
al recrear el contenedor. La rotación coordinada todavía no está implementada.

La imagen `dockurr/proxmox` está fijada por digest; el paquete `pve-manager`
observado es 9.2.11. Es una imagen de terceros, con atribución conservada.
Los logs de entrada se descartan y el access log de cómputo va a `/dev/null`
para evitar almacenar URLs con tickets; el diagnóstico queda limitado a
comprobaciones explícitas y al estado de los servicios.

## Evidencia y pendientes

Verificados: KVM API 12, cuatro servicios activos, HTTPS 200 con TLS válido,
login API y recuperación después de reiniciar `gnx-compute`.

Pendientes: reboot Windows con cómputo instalado, consola WebSocket interactiva,
VMs/LXC, acceso desde otro peer, respaldo y restauración del estado de cómputo.
El backup USB actual sólo cubre el control plane. Este despliegue comparte WSL
y red de contenedores con el control; no ofrece aislamiento para cargas hostiles.

## Referencias

- [Imagen de Proxmox de Dockur](https://github.com/dockur/proxmox)
- [Quadlet 4.9.3](https://docs.podman.io/en/v4.9.3/markdown/podman-systemd.unit.5.html)
- [Proxy HTTPS y WebSocket](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy)
