# GnX Node v0.1.57

## Cambios

- Cierra la ruta implícita de boot al consultar estado: antes de ejecutar la comparación readonly dentro de WSL, inspecciona `wsl --list --running --quiet` bajo la identidad dedicada. Un distro detenido/no accesible se proyecta como reconciliación pendiente, no se lanza para consultar el checkpoint. Aplicarlo requiere confirmación de mantenimiento.
- Sin cambio de imagen/configuración de compute frente a v0.1.56. Sigue el digest `ghcr.io/mayas-alas/tailnet-proxmox@sha256:3da7e2c7056c6fd80a5995e3486698c0a7c2563a09e3f7d3996e0f3027d7d28e` (`9.2.21-gnx.2`).
- Incluye v0.1.56: sin `pve-fake-subscription`, caché/timer falso ni `blockLicense()`/hosts loopback para `shop.maurer-it.com`; comportamiento oficial de suscripción, repos no-subscription y NAT/container adaptations legítimos conservados. Imagen/build/publicación únicamente en [tailnet-proxmox](https://github.com/mayas-alas/tailnet-proxmox), commit `8575b56`, [CI 37360545004](https://github.com/mayas-alas/tailnet-proxmox/actions/runs/37360545004).
- MSI no equivale a runtime actualizado: snapshot/PWA distinguen payload/digest vivo, Setup confirma mantenimiento y automatización usa `gnx upgrade --confirm`. Lifecycle y espera de readiness (máximo 5 minutos) externos, sin secrets/configuración de red embebidos en Rust. Sin re-enroll, password reset ni recreación cuando ya coincide el compute aplicado.

## Validación — 2026-10-05

- v0.1.56: instalado/aplicado en host existente, 28 checks canónicos pass, LXC 100 `rustfs` running, storage `local` activo y digest vivo healthy. CA (SHA-256), SID dedicado y enrolamiento/IP conservados. Arranque controlado y aceptación con espera nueva pasaron; Retry sin cambios no reinició compute.
- Computer-use Firefox en `compute.gnx`: Hosts sin el dominio bloqueado y Subscription `notfound: There is no subscription key`, no activación falsa. Paquete ausente confirmado por dpkg y lifecycle check.
- v0.1.57: 65 tests Rust aprobados, 20 casos lifecycle/hosts/warm-readiness en memoria y contratos estáticos aprobados. Parser probado con lista vacía/otros/no-running y UTF-16; guarda antes de `wsl --exec`. No se terminó manualmente el distro live para este test.
- Artefacto publicado en revisión inmutable `cd8e1c63d11e34f174fa1ccac3adc05f03710e19`: 309,679,581 bytes; SHA-256 `5e7e2b8c96ce559bbae603779a8c09e2bb462c0dc3e311af459f168a13a29c7a`, manifest remoto y descarga completa comprobados.
- Build release/MSI/Burn y verificación canónica/rootfs sellado/hashes aprobados; warning debug LNK4098 no fatal, WiX sin warnings/errors. Bundle instalado exit 0, `gnx 0.1.57`, status y aplicación explícita concluyeron `NODE_READY`. Al ser sólo host, status siguió ready (mismo payload Linux), y aplicar no cambió StartedAt de compute: no hubo otra recreación.

No se afirma clean-host enrollment ni upgrade con VMs QEMU existentes: el inventario no tiene VMs QEMU. No se purga historia Git/LFS. Se conserva el baseline funcional validado, sin otra recreación necesaria por este cambio de host.

## Descarga y aplicación

[Descargar GnxNodeSetup.exe](https://huggingface.co/datasets/gnx-labs-mx/gnx-node-releases/resolve/cd8e1c63d11e34f174fa1ccac3adc05f03710e19/releases/v0.1.57/GnxNodeSetup.exe). Burn incluye MSI verificado; hashes en `SHA256SUMS.txt`. Instalar, abrir Setup elevado y confirmar aplicación. Instalación silenciosa: después ejecutar `gnx upgrade --confirm` elevado; usar mantenimiento porque actualizar compute puede reiniciar VMs/LXC.

Los aliases `host.containers.internal`/`host.docker.internal` son normales de Podman. Sin licencia real, el aviso oficial de Proxmox es correcto; no se oculta ni se simula una licencia. Producto AGPL-3.0-only, atribución upstream MIT y licencias de dependencias conservadas. Space conserva fuente/notas/hashes y enlaza el bundle completo en el dataset de artefactos existente: el upload adicional al Space respondió `Repository storage limit reached (Max: 1 GB)` con 927,035,656 bytes de LFS histórico. No se purgó historia ni se borraron artefactos publicados; sólo se retiran directorios viejos del árbol visible. Notas históricas en `docs/releases/`.
