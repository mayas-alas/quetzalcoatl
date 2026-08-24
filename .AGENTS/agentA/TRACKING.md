# TRACKING — tabla maestra 360°

Un renglón por ticket. Vocabulario e idéntico a `FLOW.md`. Contiene el estado
de cada ticket y la **progresión ker** (el cálculo de XP/niveles vive en
`python/karma.py` a partir de estas columnas).

## Leyenda de campos

| Campo | Qué es |
|---|---|
| `ID` | `A-<NNN>` secuencial global. |
| `role` | Rol dueño: `maya`/`arquitecto`/`verificador`/`evaluador`/`juez`/`doer`/`sr`. |
| `grade` | 1 (ejecución), 2 (calidad), 3 (dirección). |
| `state` | `queued`/`claimed`/`started`/`verify`/`done`/`blocked`/`parked`. |
| `task` | Objetivo puntual y verificable. |
| `effort` | Horas reales acumuladas hasta `updated_at`. |
| `corr` | Correcciones consumidas (≤2 antes de loop-guard). |
| `started_at`/`updated_at` | Hitos UTC. |
| `checkpoint` | Último hito observable (avance). |
| `done_by` | Rol de grado ≥2 que firmó (o `-` si no está `done`). |
| `evidence` | Commit/hash/salida al cerrar. |

Los campos XP/nivel/badges se **derivan** y no se reescriben; se calculan con
`python/karma.py` y se reflejan en el bloque `Progresión` de abajo.

## Tabla maestra

| ID | role | grade | state | task | effort | corr | started_at | updated_at | checkpoint | done_by | evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A-001 | maya | 3 | done | Crear el marco `agentA` (ROLES, FLOW, GAMIFIED, TRACKING, CORRECTIONS, TEMPLATES, agents, python) | 1.5 | 0 | — | 2026-08-23 | estructura final consolidada | verificador | commit de este entregable |
| A-002 | maya | 3 | done | Enlazar `agentA` desde `.AGENTS/README.md` y `AGENTS.md`; commitear el marco (durable, sticky, sin Kilo) | 0.5 | 0 | 2026-08-23 | 2026-08-23 | marco enlazado en `.AGENTS/README.md` + `AGENTS.md`; `.gitignore` sin `agentA`; `repository.py` incluye inventario agentA; validadores ok | verificador | commit del enlace + `repository.py` ok + `karma.py --check` ok |

## Reglas de edición

- **Transición**: un renglón viaja `queued → claimed → started → verify →
  done` (o `blocked`/`parked`). No saltar pasos.
- **Corrección**: cada retrabajo incrementa `corr` y entra en
  `CORRECTIONS.md`. Al llegar `corr ≥ 3` o superar `time_box`, parar y seguir
  `FLOW.md` (loop-guard).
- **Evidencia**: `done` sin `evidence` o sin `done_by` vuelve a `verify` en el
  siguiente review.
- **Progresión**: no mutar la columna de XP aquí; se regenera con
  `python/karma.py` y se commitea.

## Progresión (regenerada por python/karma.py)

| role | nivel | XP | correcciones | esfuerzo(h) | streak clean | badges |
|---|---|---:|---:|---:|---:|---|
| maya | doer avanzado (g1) | 120 | 0 | 2.0 | 2 | - |
