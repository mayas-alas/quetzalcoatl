# v0.1.69 — candidato PWA Pocket ID

Decisión: OAuth2 Proxy + Pocket ID; grupo `gnx-app-users`; sesiones efímeras en
Valkey. Caddy mantiene TLS y el proxy autentica y sirve toda la PWA.
Sin migración NetBird ni cambios externos de política de red.

Protección de páginas/assets/status, PKCE/nonce y CA privada. Logout local POST
same-origin con destino fijo, eliminación de sesión del servidor y limpieza de
cache/storage. Se retira el shell offline antiguo. Cliente OIDC y grupo creados
idempotentemente; no se asignan personas ni se inventan passkeys.

Árbol fuente `workloads/private-services`, semántica PWA separada: máquina de
servicios privados, almacenamiento RustFS, identidad Pocket ID y red NetBird.
Tags/IPC/rutas guest heredadas se conservan por compatibilidad, sin recreación.

v0.1.68 detectó `APP_AUTH_START_FAILED`: Quadlet Podman 4.9 no admite `AddHost`.
Se reemplazó por su argumento nativo fijo, sin bajar los límites de seguridad.

**En validación.** No afirmar sesión Pocket personal, autorización por grupo o
replay de cookie real como aceptados hasta obtener evidencia correspondiente.
