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

### v0.1.72: corregido el 500 después del login Pocket

La PWA usa el `sub` firmado de Pocket y el grupo `gnx-app-users`, no depende de
un correo verificado. Conserva OAuth2 Proxy, sesiones Valkey, PKCE/nonce/CSRF
y logout seguro. El hotfix permitió acceso personal real; esta release incorpora
el fix al instalador offline y su actualización exacta llegó a **NodeReady**.
Identidad, CA, credenciales y directorios persistentes preservados.

[Descarga y hash](releases/v0.1.72/ARTIFACT.md) ·
[Notas](releases/v0.1.72/RELEASE_NOTES.md) ·
[Guía de acceso](docs/pwa-access.md) ·
[Pruebas y límites](docs/validation/v0.1.72/README.md).

66 tests Rust, 16 checks OIDC aislados, 53 checks runtime + payload actual y
6 escenarios de condiciones del MSI aprobados. **No se hizo desinstalación
destructiva real**; ni se declara logout/revocación Pocket personal o tráfico
entre peers completamente aceptados. Sin migración de la red a NetBird.

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
