# GnX Core 0.0.17

Candidata en verificación; no publicada.

- Admite redes bridge definidas por `.network` y referencias desde `.container`.
- Comprueba cada unidad solicitada, en lugar de aceptar el resultado agregado.
- Adapta las unidades de sistema de la base al guest no privilegiado.
- Incluye configuración versionada del stack privado en red bridge, sin compartir
  la red del guest entre las aplicaciones.
- Publica el stack registrado por el gateway existente, con HTTPS y CA persistente:
  Pocket ID, consola RustFS, endpoint S3 y NetBird con su dashboard.
- Configura clientes OIDC dedicados de Pocket ID para NetBird y RustFS mediante
  APIs nativas; conserva credenciales previas y no simula passkeys humanas.
- Mantiene el MSI como dueño de la instalación y el EXE como entrada interactiva.

La comprobación del login humano requiere la passkey del operador. El soporte de
redes no incluye todavía builds por Containerfile ni todas las clases de Quadlet.
Los empaquetados y los recorridos de aplicación se verifican por separado.

Límite detectado al instalar: la reconciliación de acceso puede ejecutarse antes
de que PVE termine de arrancar, provocando reinicios de compute y respuestas 502.
La espera nativa de disponibilidad se incorpora en 0.0.18; no se sustituye este CLI.
