# Contrato de agentes

- La ejecución y coordinación de agentes queda fuera del runtime del producto;
  no se implementa dentro de `gnx` ni los instaladores.
- Las interfaces públicas usan nombres GNX; las atribuciones legales se conservan.
- Tokens, claves privadas y URLs de actualización no entran en Git, argv, logs,
  capturas ni evidencia; los ejemplos contienen sólo valores no secretos.
- `legacy` es archivo histórico y no se modifica.
- Un gate fallido se reporta; nunca se oculta como éxito.
