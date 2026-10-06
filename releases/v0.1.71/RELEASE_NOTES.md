# v0.1.71 — candidato: Pocket ID para la PWA

## Alcance aprobado

OAuth2 Proxy + Pocket ID, grupo `gnx-app-users`, sesiones del servidor en Valkey.
Caddy mantiene TLS y el proxy autentica y sirve toda la PWA. Sin migración de red
a NetBird. Autoridad para agentes: `.AGENTS/30-pwa-auth.md`.

- Páginas, assets y `status.json` protegidos; PKCE S256, nonce y CA privada.
- Logout local POST same-origin, destino fijo, eliminación de sesión del servidor
  y limpieza cache/storage; sin tokens en URLs ni promesa de logout global Pocket.
- Worker anterior retirado: sin shell offline; el nuevo frontend sólo actualiza
  registrations existentes, evitando un bucle unregister/reload.
- Cliente OIDC confidencial propio y grupo creados idempotentemente; no asigna
  personas ni inventa passkeys. Tokens fuera de JS/env/argv/logs.
- Imágenes offline digest-pinned, Pull=never, procesos UID 10001, loopback y sin
  puertos underlay. Valkey no persiste sesiones: restart invalida las anteriores.
- Árbol `workloads/private-services`; PWA y CLI distinguen máquina compartida,
  almacenamiento RustFS, identidad Pocket ID y red NetBird. Nuevos aliases
  `gnx workload private-services start|stop --confirm`; IPC/tag/rutas guest
  antiguos siguen como compatibilidad, sin recreación de identidad/datos.

## Validación

- 15 pruebas con los binarios fijados de OAuth2 Proxy/Valkey y un issuer OIDC
  **aislado**, HTTPS y JWT RS256: miembro autorizado, sin grupo rechazado,
  refresco tras quitar grupo, cookie Secure/HttpOnly/host-only, callback sin
  CSRF, cookie falsa, logout/replay rechazado y caída Valkey fail-closed.
- Tests estáticos y worker retirement en memoria; imports offline aislados.
- Candidato anterior v0.1.70 instalado alcanzó NodeReady y pasó el bloqueo
  anónimo real, login HTTPS, logout CSRF, signed-out y retirement del worker.
- Firefox real llegó a Pocket **Iniciar sesión en GnX App** y al diálogo de PIN
  Windows Hello. **La persona debe completar ese paso**: no se fabricó su PIN.
- v0.1.71 exacta instalada exit 0; upgrade terminó en **NodeReady**, credenciales
  preservadas, `payload_current` pass y 53 checks del diagnóstico pass.
  La aceptación personal sigue pendiente.
- El diagnóstico general conserva warnings Proxmox ebtables/pending changes;
  este candidato de auth no los declara resueltos.

Ver [evidencia y límites](../../docs/validation/v0.1.71/README.md) y
[guía de acceso](../../docs/pwa-access.md).

Las pruebas del issuer aislado no demuestran una sesión Pocket personal. No se
promueve este candidato como aceptación completa hasta verificar sesión real,
grupos, logout y replay en el nodo instalado. Cold boot/host limpio no probados.
