# Scope aprobado: autenticación de la PWA

Decisión explícita del operador: **OAuth2 Proxy + Pocket ID**. No sustituir por
login en JavaScript, otro IdP o plugin Caddy sin una nueva decisión.

## Objetivo y límites

- Login Pocket ID para toda `https://app.gnx`: páginas, assets y `status.json`.
- Autorización explícita por `gnx-app-users`; cuenta Pocket ID no equivale a acceso.
- Caddy termina TLS y envía la PWA a OAuth2 Proxy, que autentica **y sirve** los
  archivos. No puede existir un `file_server` alternativo para contenido privado.
- Sesiones del servidor en Valkey efímero: logout borra la sesión, no sólo cookie.
- Logout local, POST del mismo origen y retorno fijo a `/signed-out`; nunca token
  OIDC en una URL de logout, ni promesa de cerrar Pocket ID/NetBird globalmente.
- TLS/CA privados obligatorios; PKCE S256, nonce, cookie Secure/HttpOnly/SameSite,
  cliente confidencial separado del cliente NetBird y secretos en archivos.
- No caché/offline de contenido autenticado. `/sw.js` es una excepción pública
  exclusivamente para retirar el worker antiguo; `/signed-out` sólo texto público.
- No red/control-plane adicional: conservar Tailscale, Pi-hole, Caddy, Named Pipe.
  NetBird no participa en el login ni cambia rutas, ACLs o enrolamiento.
- No alta automática de personas, asignación automática de grupos, credenciales
  inventadas o simulación de passkeys/Windows Hello. Administrador Pocket asigna
  usuarios al grupo deliberadamente.
- Pocket comparte la máquina de servicios privados con RustFS: apagar ese guest impide nuevas autenticaciones.
  Conservar fallo cerrado y recuperación por CLI; no despertar guests detenidos.

## Aceptación

Documentar separadamente: bloqueo anónimo, headers falsificados, sesión miembro,
usuario sin grupo, expiración/revocación, logout + replay de cookie, back-button,
worker anterior/caché, caída de IdP/proxy y HTTPS privado. Screenshots mediante
computer-use prueban sólo lo observado; un mock OIDC no prueba login Pocket real.
No publicar un candidato como aceptación completa si falta passkey real.

Implementación: `payload/node/auth/`, `services/app-auth.container`,
`services/app-sessions.container`, `workloads/private-services/app-enroll.run` y PWA.
El LXC compartido no se llama conceptualmente RustFS. `rustfs` permanece sólo
como clave IPC/tag/ruta runtime heredada para conservar la identidad existente;
RustFS es el servicio de almacenamiento, no el propietario de Pocket/NetBird.
Imágenes offline `Pull=never`, digests del import aislado y manifest dataset.
