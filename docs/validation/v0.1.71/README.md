# v0.1.71 — evidencia y límites

## Nodo instalado: observado

Burn instalado **exit 0**; `gnx version` = **0.1.71**. `gnx upgrade --confirm`
terminó en **NodeReady**, credenciales preservadas. No se parcheó manualmente
el payload de este candidato después del build. `payload_current` pasa.

- `status.json`: proyección mínima leída como administrador desde el estado
  nativo, **no obtenida mediante bypass de la URL protegida**.
- `runtime-checks.jsonl`: 53 checks pass extraídos del diagnóstico de sólo lectura
  con el host-agent instalado. Se excluyen journals/identificadores no necesarios.
- OAuth2 Proxy y Valkey loaded/active/running; guest compartido sigue siendo LXC
  100 con su tag/hostname heredados. RustFS/Pocket/NetBird healthy.
- Denegación anónima de páginas, assets y status incluso con cabeceras falsificadas;
  redirección HTTPS a Pocket; CSRF/GET/direct sign-out bloqueados; signed-out
  no-store/limpieza y worker de retiro. Todos los checks canónicos pasan.
- HTTPS/OIDC, Management/Signal gRPC, relay WebSocket y STUN también pasan;
  no demuestra tráfico real entre peers ni migración a NetBird.

El diagnóstico general conserva advertencias de Proxmox sobre ebtables no
soportado/pending changes en este kernel. Son un asunto independiente que no
queda declarado resuelto por el login PWA; no se publican journals completos.
Los controles nftables de gateway/guest y los protocolos sí se comprobaron.

## Seguridad del proxy: issuer aislado, no Pocket personal

`isolated-oidc.jsonl`: **15 tests pass**, bins reales digest-pinned, TLS validado,
JWT RS256 con discovery/JWKS, PKCE S256 y cliente confidencial. Cubre:

- Usuario test del grupo autorizado y sin grupo rechazado.
- Refresco con grupo retirado rechazado (intervalo test acelerado a 1s).
- Nonce firmado incorrecto y callback sin cookie CSRF rechazados.
- Cookie host-only Secure/HttpOnly/SameSite=lax; cookie falsa rechazada.
- Logout elimina cookie **y** sesión del servidor: replay del ticket anterior
  rechazado. Caída Valkey no sirve contenido privado.
- No tokens en el contenido de la aplicación.

Este issuer y sus claims son sólo fixtures aisladas; **no crea personas en
Pocket**, no simula una passkey humana ni mide revocación con Pocket real.
`offline-pins.txt`: imports verificados en un store vfs aislado, sin pulls.

## Otras pruebas

66 Rust tests, 7 Python de políticas/parsing, contracts estáticos, mocks LXC,
worker retirement ejecutado en memoria (GnX caches solamente, unregister y
navigate; sin fetch/offline). El frontend no registra otra vez el worker de
retiro: no genera un bucle de recarga. LNK4098 persiste en tests debug del host.

## Navegador y bloqueo humano

Firefox real llegó a **Iniciar sesión en GnX App** de Pocket ID y al diálogo
**Seguridad de Windows → Escribe tu PIN**. La captura de la página se obtuvo
con v0.1.70; véase [esa evidencia](../v0.1.70/README.md). Tras instalar v0.1.71,
el diálogo seguía esperando intervención humana. No se escribió ni solicitó
el PIN en chat ni se creó una identidad.

**Aún no demostrado en Pocket real**: login personal completado, acceso de
miembro/no miembro, refresco/revocación, logout/replay y BFCache de una sesión
real. No se presenta la pantalla de login ni los tests del issuer aislado como
aceptación de estos pasos. Tampoco se probó cold boot ni host limpio.

Para continuar, la persona completa su passkey y el administrador asigna
explícitamente el acceso a `gnx-app-users`; ver [guía](../../pwa-access.md).
