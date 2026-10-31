# v0.0.22

- Catálogo incorporado, fuentes de imágenes offline, IDs linux/amd64 y Quadlets
  coherentes: Tailscale v1.104.4, Pi-hole 2026.10.1, Caddy 2.11.7,
  NetBird server 0.82.0 y dashboard v2.96.0, fijados por digest. Se mantienen
  Pocket ID v2.19.1, RustFS 1.0.4 y compute 9.2.21-gnx.2.
- El actualizador reconoce los pins NetBird originales del stack registrado de
  0.0.20 aunque todavía no exista un recibo de actualización. Conserva la
  validación independiente de propiedad del guest; no adopta imágenes arbitrarias.
- Los LXC creados por GnX llevan el tag PVE `gnx`. Al aplicar una imagen al stack
  privado registrado se añade ese tag conservando los demás y usando el digest
  de configuración PVE para evitar sobrescribir cambios concurrentes.
  `update --check` y la descarga preparatoria no escriben tags.
- Se mantienen catálogo aprobado explícitamente, intención de parada, identidad
  y datos persistentes. No introduce inmutabilidad ni rollback de la base WSL/LXC;
  la recuperación del sistema y las migraciones de datos son capas independientes.
