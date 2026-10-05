# GnX Node v0.1.55 — candidato local, no publicado

## Cambios

- Compute consume `ghcr.io/mayas-alas/tailnet-proxmox@sha256:3da7e2c7056c6fd80a5995e3486698c0a7c2563a09e3f7d3996e0f3027d7d28e` (`9.2.21-gnx.2`). Repo de imagen commit `8575b56`, upstream `5fccb4a7`.
- Se retira `blockLicense()` y su llamada de `src/network.sh`, además de la instalación del `.deb` falso ya retirada. El bloqueo de `shop.maurer-it.com` provenía del script upstream, no del paquete. No se añade caché falsa, activación periódica ni nag bypass. Repos apt no-subscription y adaptaciones necesarias del contenedor se conservan.
- MSI/Burn instala el payload Windows; el runtime es una fase explícita. Setup detecta el desfase en lugar de ignorarlo por un `NODE_READY` previo. Pide confirmación de mantenimiento; automatización: `gnx upgrade --confirm`. No se re-enrolla, pide otra key ni resetea contraseña.
- Snapshot y PWA muestran `RUNTIME_UPDATE_PENDING` si el payload instalado no corresponde al aplicado. Consultar estado no descarga imágenes, copia payload ni reinicia guests.
- `compute/reconcile.run` es el lifecycle externo: descarga por digest antes de cambiar Quadlet, compara imagen real del contenedor y configuración aplicada, comprueba paquete ausente y hosts sin bloqueo, y sólo registra éxito tras activación. Fallos de restart no se ocultan en un Retry con Quadlet idéntico. Password rotation fuerza recreación para consumir el EnvironmentFile protegido.
- Verificador canónico exige release activo correcto, KVM, HTTPS validado por CA, gateway y workloads antes de `NODE_READY`. El runner WSL permite 15 minutos, no 5 frente a un start-timeout de compute de 8 minutos. Diagnóstico de compute ya no materializa payload ni modifica WSL/config.
- Quetzalcoatl sigue como consumidor de Quadlet; imagen/build/publicación pertenecen a GitHub. No se reincorpora un builder ni downloader OCI al producto.

## Validación — 2026-10-05

- Imagen: [GitHub Actions 37360545004](https://github.com/mayas-alas/tailnet-proxmox/actions/runs/37360545004), éxito. Build amd64, ausencia del paquete/caché y bloqueo hosts, boot PVE/HTTPS aislado y KVM antes de publicar OCI exacto con Skopeo. Digest remoto comprobado.
- Paquetes de imagen: `proxmox-ve 9.2.0`, `pve-manager 9.2.21`, `qemu-server 9.2.10`, `pve-container 6.1.14`.
- `cargo test --workspace --locked`: 64 pruebas aprobadas. Warning debug LNK4098 sin fallo; Rust release y WiX sin errores/warnings.
- Lifecycle: 14 casos reales con fakes OS en memoria y 3 casos del parser hosts: caché/no-op, contenedor viejo con imagen en caché, pull fallido sin sustitución, restart fallido sin receipt, Retry posterior, paquete/exec fallido, bloqueo de licencias, password rotation y comentarios/aliases IPv4/IPv6. Sin fixtures escritos.
- Build canónico MSI/Burn y verificación del rootfs sellado/hashes: aprobados.
- Bundle instalado con exit 0; detectó correctamente `RUNTIME_UPDATE_PENDING`. Aplicación activó el digest nuevo healthy y quitó el bloqueo hosts, pero la primera verificación falló por warm-up del LXC/gateway. Al repetir sólo el verificador, los 28 checks pasaron y `ready=true`. No se publicó este candidato: v0.1.56 incorpora la espera acotada. Inventario previo: sin VMs QEMU, LXC 100 `rustfs` running, storage `local` activo.

No se afirma alta en un host limpio ni upgrade con VMs QEMU existentes; el host de aceptación no tiene VMs QEMU. No se purga historia Git/LFS ni se cambia la CA/identidad para actualizar.

## Instalación y descarga

Este candidato sólo se instaló para aceptación local; no se publicó en nube. La release entregable es v0.1.56, que corrige la carrera observada en readiness.

Instalar, abrir Setup elevado desde el final del bundle y confirmar el update; para instalación silenciosa, ejecutar después `gnx upgrade --confirm` en terminal elevada. El runtime no se considera actualizado por instalar MSI ni por tener una imagen descargada. Compute/VMs/LXC pueden reiniciarse durante la aplicación: usar ventana de mantenimiento.

Producto AGPL-3.0-only; dependencias e imagen conservan sus licencias. Sólo la última release queda en `releases/`; Markdown histórico en `docs/releases/`.
