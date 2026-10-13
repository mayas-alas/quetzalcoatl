# v0.1.72 — Pocket passkey login: identidad por subject firmado

## Corrección

La configuración original devolvía 500 después de un login Pocket válido cuando
el ID token contenía `email_verified=false`. El diagnóstico real confirmó el
rechazo de correo no verificado. Pocket autentica por passkey; verificar un
buzón no debe ser un requisito inventado para acceder a esta PWA privada.

OAuth2 Proxy usa ahora el `sub` firmado e inmutable de Pocket en su campo interno
de identidad y exige `gnx-app-users`. No se reenvían email/identidad/tokens a la
PWA. Se conservan firma, issuer/audience, PKCE, nonce, CSRF, cookies seguras,
sesiones Valkey, revocación y logout local. No se habilitan flags de bypass.

El hotfix anterior logró acceso real en Firefox, mostrando Nodo operativo/100%
y RustFS/Pocket/NetBird operativos. Esta versión incorpora el cambio al MSI/Burn
offline numerado: no depende de un parche manual posterior al build.

## Instalación y alcance de validación

Actualización in-place: instalar el bundle y aplicar explícitamente el payload
con `gnx upgrade --confirm` elevado, en una ventana de mantenimiento. Mantiene
identidad, CA, credenciales y datos; no crea personas ni asigna grupos.

- 66 tests Rust y contratos estáticos.
- 16 checks de OAuth2 Proxy/Valkey reales con issuer OIDC **aislado**, ahora con
  correo no verificado: acceso miembro, denegación sin grupo, revocación al
  refrescar, nonce/CSRF, cookies seguras, logout/replay y fallo cerrado.
- 6 escenarios sobre condiciones del **MSI compilado**: limpieza en fresh
  install/desinstalación completa; supresión en repair, upgrade y remoción parcial.
  Ningún escenario ejecuta un custom action ni desinstala el nodo activo.

La aceptación exacta de la instalación y preservación se registra en
[evidencia](../../docs/validation/v0.1.72/README.md). Descarga y SHA-256 en
[ARTIFACT.md](ARTIFACT.md) / `SHA256SUMS.txt` tras la publicación verificada.

## Límites

No se ejecutó desinstalación destructiva en un host desechable: en el nodo activo
borraría `gnx-node`, cuenta/perfil, CA, credenciales, VMs/LXC y datos. Las pruebas
MSI no sustituyen ese ensayo end-to-end. Tampoco prueban cold boot/host limpio,
tráfico de peers reales o resolución de warnings Proxmox ebtables/pending changes.
Las pruebas aisladas de logout/revocación no equivalen a repetirlas con la sesión
Pocket personal. No hay migración de red a NetBird.
