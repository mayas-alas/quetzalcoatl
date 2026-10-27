# GnX Core 0.0.18

Pre-release candidata para Windows x64. No es una versión estable ni una
certificación de funcionamiento sin errores.

- Admite redes `.network` bridge y alias DNS en el subconjunto de Quadlets.
- Incluye definiciones del stack RustFS, Pocket ID y NetBird con su dashboard,
  sin `Network=host` para las aplicaciones y con datos persistentes en el guest.
- Publica el stack registrado por HTTPS mediante el gateway existente y configura
  clientes OIDC separados de Pocket ID para RustFS y NetBird mediante APIs nativas.
- Comprueba el estado de cada unidad solicitada, no el éxito agregado del conjunto.
- Espera disponibilidad de PVE mediante el healthcheck nativo de Podman antes de
  reconciliar la publicación del stack; no confunde contenedor iniciado con PVE listo.

## Primer acceso y límites

- Antes de iniciar sesión desde RustFS o NetBird, completar
  `https://pocket.gnx/setup` y registrar una passkey real. La cuenta técnica de
  preparación de la API no es una cuenta humana ni sustituye ese recorrido.
- El login y la autorización bajo una identidad humana requieren comprobación
  independiente; configurar OIDC o mostrar su página no acredita aceptación.
- El portproxy TCP de Windows y Caddy no transportan STUN UDP. El puerto del guest
  no acredita acceso UDP externo ni conectividad completa entre peers NetBird.
- Las imágenes de aplicaciones requieren acceso a sus registros. No se incluyen
  builds por Containerfile ni todas las clases de Quadlet.
- No se acredita instalación en un Windows limpio, mantenimiento Burn completo,
  retirada, rollback o reinstalación por superar los gates de empaquetado.
- MSI y EXE se empaquetan desde el CLI versionado mediante la Action existente;
  ese proceso no recompila el CLI ni instala los paquetes en un host de aceptación.
- Los instaladores no tienen firma Authenticode. Verificar `SHA256SUMS.txt`.

Descargas de usuario: `GnxCoreSetup.exe`, `GnxCore.msi` y `SHA256SUMS.txt`.
El CLI y la metadata técnica permanecen separados, no son otra vía de instalación.
