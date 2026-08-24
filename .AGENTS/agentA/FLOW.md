# FLOW — ciclo de trabajo + loop-guard

El motor es un ciclo **cerrado y verificable** que cualquier agente repite igual,
independiente de la herramienta.

## Estados de un ticket

| Estado | Significado | Datos mínimos |
|---|---|---|
| `queued` | A la espera de dueño. | — |
| `claimed` | Un agente lo tomó y planificó. | `agent`, `plan` (1 línea), `claimed_at` |
| `started` | En ejecución; checkpoint activo. | `checkpoint`, `started_at`, `updated_at` |
| `verify` | Listo para revisión; lo juzga un rol de grado ≥2. | salida de checks, candidato de evidencia |
| `done` | Firmado por verificador/juez + evidencia. | `evidence` (commit/hash), `done_by` |
| `blocked` | Frena por bloqueador concreto. | `blocker`, `needs`, `alternatives` |
| `parked` | Devuelto a `queued` por el loop-guard con razón. | `guard_reason` |

## El ciclo

```text
1. CLAIM   -> ticket a `claimed`; asignar agent + plan en 1 línea.
2. DO      -> ejecutar; actualizar `checkpoint` y `updated_at` en cada hito.
3. VERIFY  -> pasar a `verify`; lo **juzga** un rol de grado >=2 (no el autor).
              Si falla -> corrección: registrar CORRECTIONS.md y volver a DO.
4. RECORD  -> `done`: firmar `done_by` + `evidence` (commit/hash/salida).
5. COMMIT  -> un commit por cambio de archivo de estado, con id de ticket.
```

## Time-box por tipo de ticket

| Tipo | Time-box típico |
|---|---|
| Bugfix acotado | 1 h |
| Feature de una ruta | 2–4 h |
| Arq./contrato | 4–6 h |
| Investigación / diseño | 2 h |
| Revisión / verificación | 1 h |

Si `now - started_at > time_box` y el estado no es `verify`/`done`: **pausar**,
registrar avance real y disparar el loop-guard (park o blocked). Nunca ampliar
el time-box en silencio: solo con corrección y justificación, o aprobado por
`maya`.

## Loop-guard (anti-bucle buggy)

Objetivo: que un agente no **cicle** repitiendo el mismo retrabajo o quemando
el time-box sin progreso observable. Se dispara solo y siempre al superar un
umbral que aparece por defecto.

### Umbrales de disparo

| Condición | Acción |
|---|---|
| `corr` = 2 (2 correcciones sin `done`) | Parar; registrar en `CORRECTIONS.md`; park o blocked. |
| time_box superado | Pausar; re-estimar con corrección o park. |
| `checkpoint` congelado 2 actualizaciones seguidas | Revisar plan; nunca reintentar idéntico sin cambiar algo. |
| Misma causa raíz 2 veces | Cambiar de enfoque o escalar; no re-aplicar el mismo fix. |
| Mismo error/check falla 2 veces | Cambiar de método; registrar el error exacto. |

### Anti-patrones (prohibidos)

- "Intentar de nuevo" sin cambiar nada observable.
- "Probemos otra vez por si acaso".
- Silenciar un error desconocido.
- Aumentar el time-box sin registro.
- Retomar un `started` sin mirar `checkpoint`.
- Copiar el mismo fix dos veces.

### Secuencia obligatoria al dispararse

1. **Pausa**: parar el ticket ya; no más intentos.
2. **Registrar**: renglón en `CORRECTIONS.md` con el disparador superado.
3. **Decidir**: `parked` (razón del guardia) **o** `blocked` (bloqueador concreto
   + `needs`).
4. **Escalar**: notificar a `maya` con estado y decisión, no como pregunta
   abierta. No reclamar el mismo ticket con el mismo plan: requiere una `plan`
   nueva de 1 línea que difiera.

### Diagnóstico rápido

```text
¿checkpoint cambió en las últimas 2 actualizaciones? -> no  => park
¿es la 2ª corrección de este ticket?                 -> sí  => park o blocked
¿el error es idéntico a uno ya registrado?           -> sí  => blocked (needs ayuda)
¿time_box superado?                                  -> sí  => park (re-estimar) o blocked
```

## Reglas de transferencia

- Quien pausa deja `checkpoint`, `updated_at` y `agent_ping` al día.
- Un ticket no queda en `started` más de ~48 h sin `updated_at`.
- `blocked` sin `blocker`+`needs` se considera malformado; hay que corregirlo.
- `done` sin `evidence` o sin `done_by` de grado ≥2 vuelve a `verify` por
  defecto en el siguiente review.