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

`payload/node/workloads/private-services/offline.tar.gz` no se almacena en Git/LFS ni en
la Space. Se publica en el mismo dataset de artefactos que los instaladores:
[`gnx-labs-mx/gnx-node-releases`](https://huggingface.co/datasets/gnx-labs-mx/gnx-node-releases/tree/main/build-inputs/v0.1.60).

El build usa [`offline.url`](payload/node/workloads/private-services/offline.url), fijado a
una revisión inmutable, y verifica [`offline.sha256`](payload/node/workloads/private-services/offline.sha256).
Solo descarga si el archivo no está disponible localmente; un hash incorrecto
bloquea el build. Los guests siguen usando el bundle incluido en el instalador,
sin descargas de imágenes en runtime. Quitar el archivo de la rama actual no
reescribe los commits históricos ni garantiza que desaparezca un aviso de
seguridad de Hugging Face asociado al historial o al contenido del dataset.

## Releases

[Changelog](CHANGELOG.md) · [Release histórica v0.1.57](docs/releases/v0.1.57/RELEASE_NOTES.md)

### v0.1.71: PWA protegida, aceptación Pocket personal pendiente

Login por Pocket ID con OAuth2 Proxy, grupo `gnx-app-users` y sesiones del
servidor en Valkey. Sin migración de la red a NetBird. La máquina compartida se
llama **servicios privados**; RustFS es solamente su almacenamiento. Se conservan
los identificadores persistentes y los datos.

[Notas del candidato](releases/v0.1.71/RELEASE_NOTES.md) ·
[Guía de acceso y recuperación](docs/pwa-access.md) ·
[Pruebas y límites](docs/validation/v0.1.71/README.md).

Hay pruebas aisladas del proxy real para grupos, CSRF, nonce, logout/replay y
fallo de sesiones. No equivalen a completar una passkey personal en Pocket.

### v0.1.67: historial de disponibilidad de servicios

Este candidato alcanzó `NodeReady`: máquina de servicios privados/RustFS, Pocket ID y NetBird
combinado + dashboard operativos. El verificador real pasa HTTPS/OIDC, gRPC,
relay WebSocket y STUN; Firefox confirma el flujo NetBird → Pocket ID.

[Instalador candidato y hashes](docs/releases/v0.1.67/ARTIFACT.md) ·
[Notas y límites](docs/releases/v0.1.67/RELEASE_NOTES.md) ·
[Evidencia del nodo real](docs/validation/v0.1.67/README.md).

No se demostró aceptación personal con passkey ni peers
reales. No se presenta una pantalla de login como autenticación completada ni
un STUN binding como tráfico WireGuard.
