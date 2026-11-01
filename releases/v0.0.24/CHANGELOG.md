# v0.0.24 — Candidata

**Estado:** candidata con una corrección acotada y entrega canónica en CI.
Las comprobaciones unitarias no acreditan instalación, conservación de datos
ni recreación del nodo.

## Corregido

- Normaliza el conjunto `content` del almacenamiento PVE antes de calcular el
  hash del inventario. Un orden distinto de los mismos tipos no se interpreta
  como cambio de configuración; tipos realmente distintos siguen cambiando el hash.
- Los rechazos de revalidación indican si difieren SID, GUID WSL, guests/estados,
  configuración PVE o nombres de recibos Windows. No revelan configuraciones
  ni credenciales y no omiten la comparación.
- Dos comprobaciones unitarias acotadas en CI verifican equivalencia por orden,
  detección de cambios y rechazo de listas malformadas, sin operar el host.

## Actualización

- MSI/Burn, inventario, confirmación destructiva y límites de propiedad se conservan.
  No se añaden módulos, dependencias ni flags de build.
- El CLI instalado v0.0.23 todavía ejecuta su comparación anterior antes de lanzar
  un MSI. Puede bloquear también el salto a esta versión; publicar la corrección
  no modifica retroactivamente ese ejecutable. En ese caso usar el instalador
  nativo respetando la vía MSI/Burn existente y una ventana con guests detenidos.
