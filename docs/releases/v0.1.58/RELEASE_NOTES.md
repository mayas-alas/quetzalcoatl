# GnX Node v0.1.58

## Cambios

- Añade los Quadlets iniciales de identidad/acceso para el LXC gestionado: `pocket-id.container` y `netbird-dashboard.container`, ambos `Pull=never`, sin publicación de puertos underlay y detrás del mesh del workload.
- Publica accesos privados `https://pocket.gnx` y `https://netbird.gnx` en el gateway Caddy. El TLS termina con la CA privada del nodo; el backend viaja por `pocket.mesh.gnx:1411` y `netbird.mesh.gnx:80` dentro de Tailscale, no por LAN.
- Amplía DNS privado, hosts loopback de Windows y SAN del certificado gateway para `pocket.gnx` y `netbird.gnx`. En upgrade se regenera el leaf gateway si falta el SAN nuevo.
- Mantiene el paradigma vigente: Quetzalcoatl no descarga imágenes de workload en runtime. Las nuevas Quadlets sólo arrancan si las imágenes offline `localhost/gnx/pocket-id:1.0.0-gnx.1` y `localhost/gnx/netbird-dashboard:1.0.0-gnx.1` existen en el bundle del LXC; si no, el instalador las omite con evento explícito y conserva RustFS/compute.

## Validación

- Contratos estáticos Node/seguridad/LXC/workload aprobados con los nuevos routes y DNS: `node tests/**/*.mjs tests/*.mjs` y `bash tests/node-runtime/test-lxc-gate.sh`.
- Build release/MSI/Burn aprobado con rootfs sellado. Artefactos locales: MSI `308,981,760` bytes SHA-256 `0e5c70b068c1d3ed39b90b6f9b4069d83513f01e8860f8aad4fc65b785c75db2`; Burn `309,679,973` bytes SHA-256 `7663afbd140a98bdffe1e4c0f17f93e7493bad4e793c5df4a29eeb1f96097b36`.
- No cambia el digest de compute ni el rootfs sellado. No se abre `8006`, no se expone dashboard por underlay y no se agrega `tls_insecure_skip_verify`.

## Descarga y aplicación

[Descargar GnxNodeSetup.exe](https://huggingface.co/datasets/gnx-labs-mx/gnx-node-releases/resolve/d25929a8f01b339c12a7e03321b13f30f93b4c7a/releases/v0.1.58/GnxNodeSetup.exe). Burn incluye MSI verificado; hashes en `SHA256SUMS.txt`.

Instalar el bundle v0.1.58 y confirmar `gnx upgrade --confirm` en mantenimiento. El bundle offline del workload incluye las imágenes locales indicadas; Pocket ID y NetBird Dashboard quedan preparados para arrancar dentro del LXC tras aplicar el upgrade.
