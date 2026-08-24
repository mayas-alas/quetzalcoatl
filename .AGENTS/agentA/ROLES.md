# ROLES — escalera de 3 grados

Roles organizados en grados. Ningún rol juzga su propio trabajo. Promoción y
puntos en `GAMIFIED.md`.

## Grado 3 · Dirección

| Rol | Misión | Alcance |
|---|---|---|
| `orquestador` | Coordinador / hub 360°: asigna tickets, arbitra disputas, decide promociones y aplica el loop-guard como árbitro final. | `.AGENTS/agentA/**`, `.AGENTS/TRACKER.md` |
| `arquitecto` | Congela alcance y contrato; define taxonomías y límites de paths dueños. Preserva invariantes de `SCOPE`. | contratos, bordes de paths, taxonomía |

Definen qué se construye y quién. No ejecutan entregables de grado 1 salvo que
el propio ticket lo pida.

## Grado 2 · Calidad / juicio

| Rol | Misión | Alcance |
|---|---|---|
| `verificador` | Valida la entrega del do'er/sr: corre checks, atestigua `done`, devuelve a `verify` si falta evidencia. | evidencia, checks, `done`-gate |
| `evaluador` | Puntúa la calidad (XP/badges), contrasta esfuerzo real vs estimado y alimenta el leaderboard. | métricas, `python/karma.py` |
| `juez` | Zanjia disputas entre autor y verificador; decide corrección vs replan y lo registra en `CORRECTIONS.md`. | arbitraje, correcciones |

La firma de un `done` es de un rol de grado ≥2 **distinto** del autor.

## Grado 1 · Ejecución

| Rol | Misión | Alcance |
|---|---|---|
| `doer` | Aterriza un ticket en el ciclo cerrado (claim → do → verify → record). Primeras entregas. | tickets asignados |
| `sr` | Entrega acotada y de calidad sin supervisión; sienta el estándar y guía a `doer`s. | tickets complejos, estándar de calidad |

Un `doer` promueve a `sr` tras XP y cero correcciones en un período; un `sr`
puede ascender a grado 2 (verificador/evaluador) y más tarde a `juez` y a
dirección. Detalle en `GAMIFIED.md`.

## Reglas de la escalera

- **Un path, un dueño a la vez**; no tocar lo de otro rol.
- **Juicio separado**: el `done` de un ticket lo firma un rol de grado ≥2
  distinto del autor. `maya` es el árbitro final.
- **Tarjetas**: en `agents/` hay 5 tarjetas que cubren toda la escalera:
  `maya` (orquestador), `arquitecto`, `juez` (incluye la función de evaluador),
  `verificador`, y `doer` (incluye el rango sénior `sr`).
- **Promoción**: decisión de `maya` + datos de `python/karma.py`; nunca por
  autodeclaración.
- Se respeta la alineación de `WORKSTREAMS.md`: los roles de ejecución se
  mapean a los lanes (A/B/C) que define el repo.