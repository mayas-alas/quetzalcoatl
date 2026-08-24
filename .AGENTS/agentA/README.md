# agentA — marco de trabajo gamificado agente-agnóstico

Versión limpia y única (sin legacy, sin parches superpuestos). Un marco de
trabajo de agentes con **jerarquía de roles**, **separación ejecutor/juez**,
**seguridad anti-bucles** y **gamificación determinista** (XP/niveles/badges).
Es agnóstico: no depende de ninguna herramienta de agente (Kilo, Codex,
Claude Code, Cursor…). Cualquier agente que lea este contrato retoma el trabajo
desde el checkpoint con solo Markdown plano y Python (stdlib).

Git-friendly: tablas Markdown, un archivo de estado por flujo, referencia de
commit en cada cierre. Sin binarios ni bases de datos privadas.

## Cómo retomar el trabajo (entrada obligatoria de cualquier agente)

1. Leer `../../AGENTS.md` y el live framework (`.AGENTS/README.md`,
   `.AGENTS/SCOPE.md`, `.AGENTS/WORKSTREAMS.md`, `.AGENTS/TRACKER.md`,
   `.AGENTS/EVIDENCE.md`).
2. Leer este `README.md`, luego `ROLES.md`, `FLOW.md` y `GAMIFIED.md`.
3. Abrir `TRACKING.md` (tabla maestra). Buscar tu rol en `agents/<rol>.md` y
   tus tickets en estado `queued` / `claimed` / `started`.
4. Si un ticket quedó `started`, retomarlo desde `checkpoint` y actualizar
   `updated_at` + `agent_ping`. Nunca reclamar sin leer el checkpoint.

## Contenido

| Archivo | Qué es |
|---|---|
| `ROLES.md` | Escalera de 3 grados: Dirección, Calidad, Ejecución. |
| `FLOW.md` | Ciclo cerrado claim→do→verify→record→commit + loop-guard. |
| `GAMIFIED.md` | Motor de juego: XP, niveles, badges, streak, leaderboard. |
| `TRACKING.md` | Tabla maestra 360° (avance, esfuerzo, tiempo, XP, correcciones). |
| `CORRECTIONS.md` | Registro de errores (síntoma, causa, fix, prevención). |
| `TEMPLATES.md` | Plantillas copiables: ticket, corrección, handoff, leaderboard. |
| `agents/` | Tarjetas de rol por grado (ver `ROLES.md`). |
| `python/karma.py` | Calcula XP/nivel/leaderboard desde los `.md` (solo stdlib). |

## Los roles en 3 grados (resumen)

| Grado | Rol | Función |
|---|---|---|
| 3 · Dirección | Orquestador · Arquitecto | Planifican, asignan, definen alcance/contrato, arbitran y promocionan. |
| 2 · Calidad | Juez · Verificador · Evaluador | Prueban, puntúan, adjudican disputas, miden calidad. |
| 1 · Ejecución | Do'er · Senior Dev | Aterrizan tickets en el ciclo cerrado. |

Detalle y paths dueños en `ROLES.md`. Promoción y puntos en `GAMIFIED.md`.

## Reglas no negociables

- Trabajar **solo** dentro del ticket reclamado y sus paths dueños.
- No revertir ni sobrescribir cambios de otro agente.
- **Separación de juicio**: un agente no firma su propio `done`; lo firma un
  verificador/juez de otro grado (o `maya` como árbitro).
- **Evidencia exacta** (commit/hash/salida de check) obligatoria antes de
  `done`. Un `done` sin evidencia vuelve a `verify` automáticamente.
- Las correcciones SIEMPRE entran en `CORRECTIONS.md` antes de reintentar.
- Si un ticket cicla (≥2 correcciones o supera su time-box), activar
  `FLOW.md → loop-guard` **antes** de seguir gastando esfuerzo.
- La gamificación puntúa **calidad**, no cantidad; y **resta** XP ante
  bucles o autojuicio.

## Sesgo de escritura

- Mismo vocabulario de estados en todas las tablas.
- Fechas en UTC (`YYYY-MM-DD`); esfuerzo/tiempo en horas.
- Un cambio por archivo de estado → un commit; no mutar historia a mano.