---
name: gnx-vector
description: Recuperar dirección en GnX Core cuando la petición admite interpretaciones, crece el alcance o un encargo se desvía. Produce objetivo, exclusiones y señal de cierre; no impone planes ni multiagente a tareas directas.
---

# Vector — dirección sin expansión

Rige [`.AGENTS`](../../../.AGENTS); [AGENTS.md](../../../AGENTS.md) es el catálogo.
Entrada: petición vigente, estado/diff real y duda concreta sobre el rumbo.

## Movimiento

1. Formula el resultado que el usuario necesita, no una lista de tecnologías ni
   mejoras posibles. Contrástalo con `scope.md` y la arquitectura pertinente.
2. Separa lo pedido de lo inferido. Resuelve con evidencia las dudas de hecho;
   pregunta por intención o permisos solo si cambian el resultado o su riesgo.
3. Elige el camino más corto y reversible dentro del encargo. Descarta pasos que
   no aporten al cierre; un hallazgo lateral no se convierte en otra tarea.
4. Si conviene delegar, identifica partes independientes y el mínimo de workers.
   Por encargo: checkout/revisión y diff, contrato/catálogo, objetivo, exclusiones,
   rutas editables, permisos, evidencia esperada y presupuesto de tiempo/intentos/contexto
   según la continuidad definida en `.AGENTS`.
   Sin avance o al agotar el presupuesto: devolver pendiente y dato faltante,
   sin relanzamientos automáticos. Un dueño por archivo; escrituras compartidas en serie.
5. Verifica que cada worker accede al marco y skills requeridos: archivos locales
   sin commit no se propagan a otro checkout. La autorización no se hereda por nombre.

## Enlaces por necesidad

| Necesidad real | Siguiente recurso |
| --- | --- |
| Ejecutar delegación autorizada | `orchestration` y su guía vigente; no recrear su protocolo. |
| Operar terminal/worktree o transferir ownership | `orca-cli`; crear un worktree requiere permiso propio. |
| Falta una capacidad, no solo una instrucción | `find-skills` para investigar; no instalar por iniciativa. |
| Duda sobre proveedor/modelo/variante o configuración | `opencode`; comprobar disponibilidad sin fijarla en esta skill. |
| Una suposición cambia el rumbo / el siguiente paso cambia el riesgo | [Fricción](../gnx-friccion/SKILL.md) / [Umbral](../gnx-umbral/SKILL.md), solo para esa duda. |

## Salida y fin

**Objetivo / fuera de alcance / próximo paso mínimo / señal de cierre.**
Añade dueño y presupuesto solo si hay delegación. Reabre el vector únicamente
si cambia la petición, aparece una dependencia decisiva o el trabajo se desvía.
