# v0.1.68 — candidato PWA Pocket ID

Decisión: OAuth2 Proxy + Pocket ID; grupo `gnx-app-users`; sesiones efímeras en
Valkey. Caddy mantiene TLS y el proxy autentica y sirve toda la PWA.
Sin migración NetBird ni cambios externos de política de red.

Protección de páginas/assets/status, PKCE/nonce y CA privada. Logout local POST
same-origin con destino fijo, eliminación de sesión del servidor y limpieza de
cache/storage. Se retira el shell offline antiguo. Cliente OIDC y grupo creados
idempotentemente; no se asignan personas ni se inventan passkeys.

**En validación.** No afirmar sesión Pocket personal, autorización por grupo o
replay de cookie real como aceptados hasta obtener evidencia correspondiente.
