# v0.1.70 — candidato PWA Pocket ID

OAuth2 Proxy + Pocket ID; grupo `gnx-app-users`; sesiones efímeras en Valkey.
Caddy mantiene TLS y el proxy autentica y sirve toda la PWA. Sin migración
NetBird ni cambios externos de política de red. Alcance: `.AGENTS/30-pwa-auth.md`.

Protección de páginas/assets/status, PKCE/nonce y CA privada. Logout local POST
same-origin con destino fijo, eliminación de sesión del servidor y limpieza de
cache/storage. Se retira el shell offline antiguo. Cliente OIDC/grupo creados
idempotentemente; no se asignan personas ni se inventan passkeys.

Árbol fuente `workloads/private-services`; PWA separa máquina compartida,
almacenamiento RustFS, identidad Pocket ID y red NetBird. Tags/IPC/rutas guest
heredadas se conservan por compatibilidad, sin recreación.

v0.1.68: Quadlet Podman 4.9 no admite AddHost; corregido con argumento nativo.
v0.1.69: proxy activo y bloqueo anónimo, pero signed-out 404: upgrade copiaba
routes sin reconciliar nuevos mounts de gateway. Corregido sin bypass de auth.

**En validación.** No afirmar sesión Pocket personal, autorización por grupo o
replay de cookie real como aceptados hasta obtener evidencia correspondiente.
