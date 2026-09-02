# Contrato de agentes

Para cambios de código o arquitectura, ejecutar el ciclo acotado descrito en
`.agent/gauntlet.md`. Aplicar sólo los puntos relevantes al cambio actual.

- El agente principal actúa como Lead: fija aceptación, divide y cierra.
- Builder produce el cambio; Critic intenta refutarlo y no lo edita.
- Un mismo pase no puede construir y autocertificarse.
- Las interfaces públicas usan nombres GNX; las atribuciones legales se conservan.
- `legacy` es archivo histórico y no se modifica.
- Un gate fallido se reporta; nunca se oculta como éxito.
