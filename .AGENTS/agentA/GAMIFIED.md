# GAMIFIED — motor de juego

Determinista, reproducible y computable por `python/karma.py` desde los
`.md`. Puntúa **calidad y colaboración**, no solo cantidad de cifras.

## Puntos (XP)

| Acción | XP | Nota |
|---|---|---|
| `done` firmado por grado ≥2 | +40 | Base; requiere `evidence` |
| Entrega sin correcciones | +20 | Bonificador de una pasada |
| Antes del time-box | +10 | Proactividad |
| Verificación que atrapa un bug real | +15 | "Cazador" (para quien verifica) |
| Evaluación que detecta bucle/overrun | +15 | (para quien evalúa) |
| Mentoría de un `doer`→`sr` en el ticket | +10 | aprobado por `maya` |
| Revisión de arquitectura aplicada | +20 | (para `arquitecto`) |
| Bloqueo bien registrado que destraba a otro | +5 | ayuda cruzada |
| Auto-firma del propio `done` | **-40** | prohibido, resta |
| Corrección introducida por bucle/overrun | **-15/corr** | aplica por cada corrección |
| Reintento del mismo fix | **-20** | anti-pattern |

Los negativos vienen de `LOOP-GUARD`; la malpraxis de autojuicio siempre resta.

## Niveles (progresión)

La XP acumulada determina el nivel y desbloquea grado en la escalera:

| Nivel | XP acumulada | Grado alcanzable |
|---|---|---|
| 1 | 0–99 | `doer` |
| 2 | 100–249 | `doer` avanzado |
| 3 | 250–499 | `sr` (aprobación `maya`) |
| 4 | 500–899 | `verificador` / `evaluador` (grado 2) |
| 5 | 900+ | `juez`, y candidatura a `arquitecto`/`orquestador` (grado 3) |

Promoción = nivel alcanzado **+ decisión de `maya`**, nunca solo por número.
No hay autopromoción.

## Badges (logros)

| Badge | Condición |
|---|---|
| 🎯 `one-shot` | 3 `done` seguidos sin correcciones |
| ⏱️ `on-time` | 3 entregas dentro del time-box |
| 🛡️ `bug-hunter` | 3 verificaciónes que atraparon bug real |
| 🧭 `architect` | Plan de arq. aplicado y aceptado |
| 🧑🏫 `mentor` | 3 mentorías `doer`→`sr` acreditadas |
| 🤝 `unblocker` | 3 bloqueos `blocked` resueltos con `needs` |
| 🔁 `rebounder` | recuperarse de 2 `parked` y seguir la estrecha |

Badges se otorgan automáticamente al cumplir la condición (los cuenta
`python/karma.py` desde el histórico).

## Streak

- **Streak de entrega limpia**: n.º de `done` consecutivos sin correcciones.
  >5 → muestra "on fire" en el leaderboard.
- **Streak de revisión**: n.º de revisiones consecutivas sin bucle detectado.
- Al romper (corrección o bucle), el streak vuelve a 0.

## Incentivos / reconocimiento

- **Títulos** derivados del nivel (p. ej. `doer` → `sr` → `verificador` →
  `juez` → `arquitecto`).
- **Prioridad en handoffs**: mayor XP pide primero los tickets jugosos.
- **Autoridad de diseño**: solo grado 3 congela alcance/contrato.
- El **leaderboard** (Top N por XP del período) se regenera y se commitea en
  cada cierre; es texto plano git-friendly.

## Normas del motor

- Un agente **nunca** puntúa su propio ticket.
- Las puntuaciones las aplica `evaluador` (grado 2) y las valida `maya`.
- Sin evidencia → sin `done` → sin XP. Los negativos se liquidan aunque el
  ticket queme.
- Regenerar `python/karma.py` tras cambios de estructura salvo que esté en
  `python/` (se actualiza junto al pipeline).