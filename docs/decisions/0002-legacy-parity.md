# 0002 — Paridad con legacy: recuperar comportamiento, no portar código

- **Estado:** aceptada
- **Contexto:** rama `legacy` (snapshot 2026-08-31), AGENTS.md §5
- **Tags:** legacy, parity, scope, roadmap

## Contexto

La rama `legacy` conserva el producto anterior (Quetzalcoatl Next, corte
2026-08-31): instalador autoextraíble (EXE/AppImage), CLI de ocho subcomandos,
servicio Windows con cuenta aislada y tray, journal JSONL, **Headscale soberano**
como `login-server`, Docktail, OpenTofu dentro de un LXC runner y Podman
Machine. La base activa (GNX 0.2) es deliberadamente más pequeña: tres
capacidades, Tailscale SaaS como source of truth, dnsmasq Split DNS, Caddy con
CA opcional y un puente Windows delgado.

La pregunta de paridad: qué extraña el legacy a la nueva base y qué se recupera.

## Matriz de brechas

| Capacidad (legacy) | Estado en GNX 0.2 | Decisión |
|---|---|---|
| Instalador EXE/AppImage sin argumentos | bundle `dist/` + `install-host.ps1`/`install.sh` con checksums | **Suficiente.** No portar el empaquetador. |
| `gnx doctor` (diagnóstico global) | `compute status`, `controller status`, `access dns` (gates por capacidad) | **Recuperar sin código nuevo:** un `doctor` futuro compone los tres gates existentes. |
| `gnx logs` (journal JSONL propio) | systemd journal + `LogDriver=none` en contenedores | **No portar.** `journalctl -u gnx-*` ya cubre la trazabilidad; menos código, integración nativa. |
| `gnx repair` | `apply` idempotente (`install()` con marcador `# Managed by GNX`, `ca.sh` idempotente) | **Ya existe:** reparar = reejecutar `apply`. Documentar, no implementar. |
| `gnx update --from --sha256` | ausente | **Fuera de alcance** por AGENTS.md §3 (URLs de actualización no entran al producto). Distribución = reinstalar bundle verificado. |
| `gnx uninstall` | ausente | **Diferido** (ADR futura si se requiere). Los Quadlets son marcados y removibles sin estado huérfano. |
| Servicio Windows + cuenta `gnx-runtime` + tray | puente delgado sin estado | **Descartado por diseño:** Windows no mantiene runtime; la superficie de ataque y el código desaparecen. |
| Headscale soberano (`*.node.gnx` propio) | Tailscale SaaS + Services (`*.ts.net`) | **Descartado para el MVP:** Tailscale es el source of truth de identidad, TLS y transporte (decisión del operador, coherente con ADR 0001). Headscale puede reabrirse como ADR independiente si exigir soberanía total. |
| OpenTofu + provider bpg/proxmox en LXC runner | ausente | **Diferido:** el MVP gestiona un nodo; los workloads declarativos son un producto distinto. Requiere ADR propia. |
| Podman Machine (Windows) | Podman nativo en WSL2 | **Mejorado:** una capa menos, mismos gates. |
| Docktail dentro de Proxmox | Tailscale Services en el host | **Sustituido:** TLS administrado y nombres estables sin self-hosting. |
| Split DNS `.gnx` (dnsmasq) | presente | **Nuevo** en 0.2. |
| CA autónomo `.gnx` explícito | presente | **Nuevo** en 0.2 (ver ADR 0001). |
| Imágenes fijadas por digest | presente | **Nuevo** en 0.2. |
| Contrato de salida `READY`/`FAILED` con tests | presente | **Nuevo** en 0.2. |

## Decisión

1. **No portar código legacy a la base activa** (AGENTS.md §5). La paridad se
   evalúa por comportamiento observable, no por equivalencia de superficie.
2. **Recuperar lo que falta con integraciones, no con paradigmas nuevos:**
   - `doctor` = composición de los tres gates existentes (una función, sin
     subsistema nuevo).
   - `repair` = reejecución de `apply` (idempotencia ya garantizada).
   - `logs` = `journalctl` (systemd ya es el journal).
3. **Aceptar los descartes:** sin tray, sin servicio Windows, sin update
   automático, sin Headscale ni OpenTofu en el MVP. Cada descarte reduce
   superficie de mantenimiento; cada uno puede reabrirse con su propia ADR.

## Consecuencias

- El gap real de paridad se reduce a **un `doctor` sintáctico** (~20 líneas de
  composición) y a **documentación** de repair/logs.
- La base activa permanece < 1 200 LOC de orquestación.
- Las capacidades soberanas (Headscale, OpenTofu) quedan archivadas en `legacy`
  como referencia de diseño, no como deuda.

## References

- `legacy:IMPLEMENTATION-TRACKER.md` — corte y evidencia del producto anterior.
- `legacy:README.md` — superficie CLI del legacy.
- ADR 0001 — CA autónomo: renovación automática, raíz inmutable.
- AGENTS.md §5 — la rama `legacy` no se modifica; la base activa no carga su
  código.
