# GnX Node v0.1.67 — candidato con servicios privados operativos

## Cambios

- Pocket ID, **NetBird server combinado** (Management/Signal/Relay/STUN) y **dashboard separado** en el LXC gestionado existente, junto a RustFS.
- Imágenes incluidas en el bundle offline, fijadas al digest importado, `Pull=never`, Quadlets rootless y datos persistentes. No se publican puertos en LAN/Internet.
- Caddy/private CA: `pocket.gnx` y `netbird.gnx`; routing separado de gRPC h2c, API/OAuth2/relay y dashboard.
- Alta idempotente de cliente confidencial Pocket ID y provider en el IdP integrado de NetBird; secretos en archivos protegidos, sin reset de identidades ni adopción de owners ajenos.
- Upgrade reconcilia acceso en el guest **ya encendido**, después de activar gateway; no crea ni despierta guests detenidos.
- Preflight de permisos mesh bidireccionales, forwarding STUN limitado, diagnóstico sanitizado y errores JSON resistentes a whitespace.
- Health local del backend por `/api/instance`; readiness real exige además gRPC HTTP/2, relay WebSocket y STUN por el gateway. Se corrigió parsing de headers curl y la respuesta válida de Signal del backend fijado.
- PWA muestra Pocket ID y NetBird a partir del estado real, no de plantillas.
- `offline.tar.gz` se distribuye en el mismo dataset de artefactos que el EXE, no en el árbol actual del Space. Build verifica SHA-256 del input fijado.

## Validación instalada

- Burn/MSI v0.1.67 construido e instalado: exit `0`.
- `gnx upgrade --confirm` y consulta posterior: **NodeReady**, credenciales conservadas.
- LXC/RustFS `running/healthy`; Pocket ID y NetBird `healthy`.
- HTTPS/OIDC/dashboard, Management/Signal gRPC, relay WebSocket y STUN binding: **pass** en el verificador canónico del nodo real.
- Firefox: PWA con servicios operativos, Pocket ID, NetBird ofreciendo Pocket ID, y redirección real a la pantalla Pocket **Iniciar sesión en GnX NetBird**.
- 66 tests Rust; 7 tests Python de ACL/protocolo; contratos estáticos; test LXC mock. Import offline aislado de los tres digests validado. Warning LNK4098 observado.
- [Evidencia y límites](../../validation/v0.1.67/README.md).

## Límites y acción personal

**No se afirma login personal completo, autorización administrativa de una persona ni tráfico WireGuard entre peers.** Se necesita tu passkey real; no se simuló Windows Hello.

1. Completa tu alta en `https://pocket.gnx/setup`.
2. Abre `https://netbird.gnx` y elige **Continue with Pocket ID**.
3. Antes de enrolar peers, revisar solapamiento de rangos NetBird/Tailscale y autorizar UDP 3478 de los clientes concretos hacia el gateway. El STUN demostrado es gateway → LXC, no un cliente externo aún.

Validación de upgrade del nodo existente; instalación limpia y cold boot no demostrados. Los candidatos v0.1.60–v0.1.66 que fallaron no se promovieron.

## Distribución

EXE completo con MSI embebido: `candidates/v0.1.67/` del dataset `gnx-labs-mx/gnx-node-releases`; este directorio conserva hashes, notas y enlace inmutable. Se publica como **candidato probado de disponibilidad**, no como aceptación end-to-end completa. Fuente y evidencia: Space `gnx-labs-mx/quetzalcoatl`. No se duplica el MSI en el dataset ni se reescribe historial para limpiar LFS histórico.
