# v0.0.21

- Auditoría del updater: el comportamiento es el diseñado — el catálogo v0.0.20
  estaba fijado por digest a versiones anteriores (netbird-server 0.81.5,
  netbird-dashboard v2.95.0) mientras upstream ya publicaba 0.82.0 / v2.96.0.
  No hay fallo de actualización: `latest` no se descubre; entra por catálogo
  aprobado en una release versionada.
- Catálogo incorporado refrescado a las últimas estables verificadas contra los
  registros (2026-10-31): netbird-server 0.82.0, netbird-dashboard v2.96.0,
  mesh v1.104.4, dns 2026.10.1, gateway Caddy 2.11.7. Sin cambio: pocket-id
  (pin ya es latest v2.19.1), rustfs 1.0.4 (sigue siendo la más nueva), compute
  (pin ya es 9.2.21-gnx.2, build de la serie gnx más nueva; el tag movible
  `latest` de ghcr no se adopta por diseño).
- Etiquetado PVE: los guests creados por GnX llevan el tag `gnx` desde la
  creación (`pct create --tags gnx`) y `update --apply` lo añade después de
  validar ownership (description `GnX.Core:<digest>`, unprivileged, MAC
  registrada), preservando los demás tags existentes. Guests que fallan la
  validación —incluidos los de usuario, como los LXC propios del operador—
  nunca se tocan.
- Los guests 101/102 creados fuera del producto presentan un tag `gnx` aplicado
  externamente; retirarlo es una operación manual del operador en PVE y no
  corresponde a esta release.
- No cambia el formato del catálogo ni los comandos; el flujo
  `update --check|apply` y las condiciones de ventana de compute se mantienen.
