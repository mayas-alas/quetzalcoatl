# LXC compartido de servicios privados

Este bundle no representa sólo RustFS. Incluye:

- `rustfs.container` / `Containerfile`: almacenamiento de objetos.
- `pocket-id.container` / `app-enroll.run`: identidad y cliente/grupo de GnX App.
- `netbird-server.container` / `netbird-dashboard.container`: red privada adicional.
- `install.run`, `mesh.*`, `workload-mesh.*`: instalación/red del guest compartido.
- `offline.*`: input sellado de paquetes/imágenes, nunca descarga desde el guest.

OAuth2 Proxy y Valkey viven junto al gateway, en `payload/node/auth/` y
`payload/node/services/app-*.container`, no dentro de este LXC.

## Compatibilidad, no denominación funcional

Se conservan `gnx-rustfs` (tag ownership), `rustfs` (clave IPC/host URI), las rutas
`/opt/gnx/workloads/rustfs` y `/var/lib/gnx-node/workloads/rustfs`, así como los
volúmenes reales `/var/lib/gnx/rustfs`. Renombrar fuente no debe recrear la máquina,
rotar mesh/CA/secretos ni mover datos. Nuevos agentes no deben interpretar el
interruptor heredado como encendido exclusivo del almacenamiento: controla el
**guest completo**, incluido el login Pocket ID.
