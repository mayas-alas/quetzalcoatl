---
title: Quetzalcoatl
emoji: 🌖
colorFrom: blue
colorTo: pink
sdk: static
pinned: false
license: agpl-3.0
---

GnX Node: servicio Windows, runtime WSL2 y acceso privado a compute.

## Licencia

GNU Affero General Public License v3.0 únicamente (`AGPL-3.0-only`).
Texto completo: [LICENSE](LICENSE). Las dependencias e imágenes de terceros
conservan sus respectivas licencias y avisos.

## Archivo offline de workloads

`payload/node/workloads/rustfs/offline.tar.gz` no se almacena en Git/LFS ni en
la Space. Se publica en el mismo dataset de artefactos que los instaladores:
[`gnx-labs-mx/gnx-node-releases`](https://huggingface.co/datasets/gnx-labs-mx/gnx-node-releases/tree/main/build-inputs/v0.1.60).

El build usa [`offline.url`](payload/node/workloads/rustfs/offline.url), fijado a
una revisión inmutable, y verifica [`offline.sha256`](payload/node/workloads/rustfs/offline.sha256).
Solo descarga si el archivo no está disponible localmente; un hash incorrecto
bloquea el build. Los guests siguen usando el bundle incluido en el instalador,
sin descargas de imágenes en runtime. Quitar el archivo de la rama actual no
reescribe los commits históricos ni garantiza que desaparezca un aviso de
seguridad de Hugging Face asociado al historial o al contenido del dataset.

## Releases

[Changelog](CHANGELOG.md) · [Últimos instaladores](releases/v0.1.57/RELEASE_NOTES.md)
