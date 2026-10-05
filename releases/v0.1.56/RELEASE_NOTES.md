# GnX Node v0.1.56

## Cambios

- Imagen `9.2.21-gnx.2`: `ghcr.io/mayas-alas/tailnet-proxmox@sha256:3da7e2c7056c6fd80a5995e3486698c0a7c2563a09e3f7d3996e0f3027d7d28e`. Repo de imagen commit `8575b56`, upstream `5fccb4a7`.
- Dos remociones acotadas: instalación de `pve-fake-subscription` y función/llamada `blockLicense()` del script de red upstream. El paquete generaba `/etc/subscription` y un timer; el script añadía el bloqueo hosts independientemente. Sin caché falsa, activación periódica ni nag bypass. Repos oficiales no-subscription, adaptaciones necesarias de contenedor y NAT upstream se conservan.
- MSI instala el payload Windows; no equivale a runtime aplicado. Snapshot/PWA detectan `RUNTIME_UPDATE_PENDING`, incluso desde un checkpoint de recuperación, sin copiar, descargar ni reiniciar al consultar estado. Setup pide confirmación; automatización elevada: `gnx upgrade --confirm`. Sin re-enroll ni nueva key/contraseña.
- Lifecycle externo `compute/reconcile.run`: descarga por digest antes de sustituir Quadlet, compara imagen viva y configuración aplicada, comprueba paquete y hosts y sólo registra éxito tras activación. Caché/Quadlet idéntico no oculta un contenedor viejo ni un restart fallido. Password rotation fuerza recreación para consumir su EnvironmentFile protegido.
- `wait-ready.run` espera hasta 5 minutos al verificador canónico tras activar: LXC DHCP/mesh/gateway son asíncronos. No reinicia ni re-enrolla para superar un transitorio; timeout explícito `VERIFY_TIMEOUT`. El candidato local v0.1.55 descubrió la carrera en aceptación live y no se publicó.
- Runner WSL permite 15 minutos, no 5 frente al start-timeout de compute de 8 minutos. Diagnóstico de compute ya no materializa payload/config.
- Producto mínimo consumidor de Quadlet; imagen/build/publicación en GitHub, sin reincorporar builders ni OCI importers a Quetzalcoatl.

## Validación — 2026-10-05

- Imagen: [GitHub Actions 37360545004](https://github.com/mayas-alas/tailnet-proxmox/actions/runs/37360545004), éxito. Build amd64, ausencia del paquete/caché y bloqueo hosts, boot PVE/HTTPS aislado y KVM, publicación OCI exacto con Skopeo y digest remoto comprobado.
- Paquetes: `proxmox-ve 9.2.0`, `pve-manager 9.2.21`, `qemu-server 9.2.10`, `pve-container 6.1.14`.
- Unit/static/behavioral gates: 64 Rust, 14 lifecycle, 3 parser hosts y 3 warm-readiness; casos en memoria, sin fixtures escritos.
- Aceptación v0.1.55 candidata: bundle instaló correctamente y mostró update pendiente. Activación cambió al digest nuevo healthy, sin bloqueo hosts. La primera verificación falló por warm-up; después los 28 checks canónicos pasaron. v0.1.56 añade la espera en la ruta real de aceptación.
- Build Rust release/MSI/Burn canónico y rootfs sellado/hashes: aprobados. 64 tests Rust y todos los contratos estáticos aprobados; warning debug LNK4098, sin errores/warnings WiX.
- Bundle v0.1.56 instalado con exit 0: `gnx version` correcto, snapshot detectó update pendiente y `gnx upgrade --confirm` concluyó en `NODE_READY`. Un arranque controlado posterior también concluyó en `NODE_READY` con la espera nueva.
- Backend: digest vivo exacto `3da7e2c7…`, healthy, paquete falso y overrides hosts ausentes, 28 checks canónicos pass. LXC 100 `rustfs` running, storage `local` activo y HTTPS RustFS/PVE/app validado por CA. No hay VMs QEMU en el inventario.
- Identidad Windows dedicada (SID), CA (SHA-256) y enrolamiento/IP comparados antes/después y conservados. Retry sin cambios dejó el mismo StartedAt de compute: no recreó el contenedor.
- Computer-use en Firefox, `https://compute.gnx`: Hosts actualizado sin `shop.maurer-it.com`; Subscription muestra `notfound: There is no subscription key`, no YajuuSenpai ni una activación falsa. Los valores viejos del tab sin refrescar no representaban el backend actual.

No se afirma alta en un host limpio ni upgrade con VMs QEMU existentes: el host de aceptación no tiene VMs QEMU. No se purga historia Git/LFS ni se cambia CA/identidad para actualizar.

## Instalación y descarga

[Descargar GnxNodeSetup.exe](https://huggingface.co/spaces/gnx-labs-mx/quetzalcoatl/resolve/main/releases/v0.1.56/GnxNodeSetup.exe). Burn contiene el MSI exacto verificado; no se duplica el MSI separado. Comprobar `SHA256SUMS.txt`.

Instalar, abrir Setup elevado al final del bundle y confirmar el update. En instalación silenciosa, ejecutar después `gnx upgrade --confirm` en terminal elevada. Compute/VMs/LXC pueden reiniciarse durante la aplicación; usar mantenimiento. Los aliases `host.containers.internal`/`host.docker.internal` son normales de Podman, no bloqueo de licencias.

Sin una licencia real, el aviso oficial de Proxmox es esperado; no debe simularse una suscripción activa ni ocultarse el aviso. Producto AGPL-3.0-only; dependencias conservan licencias originales. Sólo última release en `releases/`; Markdown histórico en `docs/releases/`.
