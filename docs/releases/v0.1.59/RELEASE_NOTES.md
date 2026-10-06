# GnX Node v0.1.59

## Cambios

- Añade los Quadlets iniciales de identidad/acceso para el LXC gestionado: `pocket-id.container` y `netbird-dashboard.container`, ambos `Pull=never`, sin publicación de puertos underlay y detrás del mesh del workload.
- Publica accesos privados `https://pocket.gnx` y `https://netbird.gnx` en el gateway Caddy. El TLS termina con la CA privada del nodo; el backend viaja por `pocket.mesh.gnx:1411` y `netbird.mesh.gnx:80` dentro de Tailscale, no por LAN.
- Amplía DNS privado, hosts loopback de Windows y SAN del certificado gateway para `pocket.gnx` y `netbird.gnx`. En upgrade se regenera el leaf gateway si falta el SAN nuevo.
- Mantiene el paradigma vigente: Quetzalcoatl no descarga imágenes de workload en runtime. El bundle offline del LXC ahora incluye y carga `localhost/gnx/pocket-id:1.0.0-gnx.1` y `localhost/gnx/netbird-dashboard:1.0.0-gnx.1`; la aplicación falla de forma explícita si faltan.

## Validación

- Contratos estáticos Node/seguridad/LXC/workload aprobados con los nuevos routes y DNS: `node tests/**/*.mjs tests/*.mjs` y `bash tests/node-runtime/test-lxc-gate.sh`.
- Build release/MSI/Burn aprobado con rootfs sellado. Artefactos locales: MSI `390,668,288` bytes SHA-256 `54da0304cfe2713ff5a3d5b169a0e85c29ab57bb4efd373da4596f0109e92bbb`; Burn `391,367,529` bytes SHA-256 `072f016f97039cecbc76d94b9f5adeed78135bc54211b7b1ddd01950c39f7d5a`.
- No cambia el digest de compute ni el rootfs sellado. No se abre `8006`, no se expone dashboard por underlay y no se agrega `tls_insecure_skip_verify`.

## Descarga y aplicación

[Descargar GnxNodeSetup.exe](https://huggingface.co/datasets/gnx-labs-mx/gnx-node-releases/resolve/PENDING/releases/v0.1.59/GnxNodeSetup.exe). Burn incluye MSI verificado; hashes en `SHA256SUMS.txt`.

Instalar el bundle v0.1.59 y confirmar `gnx upgrade --confirm` en mantenimiento. El bundle offline del workload incluye las imágenes locales indicadas; Pocket ID y NetBird Dashboard quedan preparados para arrancar dentro del LXC tras aplicar el upgrade.
