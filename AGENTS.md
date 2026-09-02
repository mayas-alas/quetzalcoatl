# Contrato de agentes

- La ejecución de agentes usa el gateway HTTP local descrito en
  `docs/agent-gateway.md`; no forma parte del runtime del producto.
- No implementar coordinación de agentes dentro de `gnx`, `gnx-netd` ni los
  instaladores.
- Las interfaces públicas usan nombres GNX; las atribuciones legales se conservan.
- `legacy` es archivo histórico y no se modifica.
- Un gate fallido se reporta; nunca se oculta como éxito.
