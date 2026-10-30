---
name: gnx-umbral
description: Decidir si GnX Core tiene evidencia y autorización para avanzar, delegar, mutar o declarar éxito. Usar en cambios de riesgo y cierres comprobables; permite acotar o detener sin inventar garantías ni añadir validaciones ajenas al encargo.
---

# Umbral — pasar solo lo que está sostenido

Rige [`.AGENTS`](../../../.AGENTS); [AGENTS.md](../../../AGENTS.md) es el catálogo.
Entrada: siguiente acción o afirmación, evidencia disponible y autorización vigente.

## Movimiento

1. Identifica qué frontera se cruza: lectura a mutación, trabajo propio a delegación,
   local a remoto/publicación, o evidencia parcial a afirmación de éxito. Evalúa
   tanto el permiso como los efectos reales; una skill permitida no es un sandbox.
2. Para una afirmación, elige evidencia que la demuestre, no solo que sea compatible
   con ella. Usa comprobaciones canónicas, lee salida pertinente completa y código
   de salida; vincula evidencia a sus inputs/revisión y revalida si cambiaron.
3. Para una acción, comprueba ownership, permisos y reversibilidad antes de ejecutarla.
   Si hay resultado desconocido de una operación previa, inspecciona antes de repetir.
4. En cierre documental/marco, revisa diff incluidos archivos nuevos, enlaces y
   configuración; no reconstruyas GNX. En entrega binaria, remite a `docs/packaging.md`
   y `scripts/build.ps1`, sin copiar gates ni confundir empaquetado con aceptación.
   Distingue CI online de build en el host con inputs locales y compilación de
   fuentes de reutilización del CLI. Reutiliza la Action existente; generar o editar
   su YAML no acredita ejecución ni publicación y no autoriza limpiar el host.
5. Elige: **avanzar** si basta evidencia y permiso; **acotar** si solo se sostiene
   parte de la conclusión; **detener** si falta permiso, hay contradicción decisiva
   o no puede conocerse el efecto. No añadas operaciones para disimular el límite.

## Enlaces por necesidad

| Frontera | Recurso |
| --- | --- |
| Delegación autorizada o cierre de workers | `orchestration`; revisar fuentes/diffs, no aceptar «hecho» como prueba. |
| Handoff, terminal o worktree | `orca-cli`; no sortear una denegación con otra herramienta. |
| Configuración, registro o permisos efectivos de skills | `opencode`; registro no acredita carga en todos los workers. |
| Error de OpenCode que el usuario quiere reportar | `report`; diagnóstico y borrador, publicación solo confirmada. |
| Falta evidencia decisiva / el resultado ya no coincide con lo pedido | [Fricción](../gnx-friccion/SKILL.md) / [Vector](../gnx-vector/SKILL.md), solo sobre el bloqueo. |

## Salida y fin

**Decisión y motivo / comprobación / límite / bloqueo.** Para trabajo comprobable,
marca resultados como **comprobado**, **fallido**, **no realizado** o **bloqueado**.
No equipares compilado, empaquetado, instalado, aplicado y aceptado. No presentes
un control de instrucciones como aislamiento técnico ni una limitación como éxito.
