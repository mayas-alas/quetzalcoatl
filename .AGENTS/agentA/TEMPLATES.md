# TEMPLATES — plantillas copiables

Mismo vocabulario que `FLOW.md` para que `TRACKING.md` quede comparable.

## 1. Ticket de tarea

```markdown
## A-<NNN> — <objetivo puntual>
- role: <maya|arquitecto|verificador|evaluador|juez|doer|sr>
- grade: <1|2|3>
- zones: <paths que se tocarán, dueño>
- goal: <qué exactamente se entrega; señal de terminado verificable>
- time_box: <N h>
- depends_on: <ID o ->
- checklist:
  - [ ] CLAIM  (agent + plan en 1 línea)
  - [ ] DO     (checkpoint en cada hito)
  - [ ] VERIFY (salida de checks guardada; juzga grado >=2)
  - [ ] RECORD (evidence + done_by, firmado por otro rol)
  - [ ] CLOSE  (done | blocked | parked, con señal)
```

## 2. Renglón de avance en TRACKING

```markdown
| A-<NNN> | <role> | <grade> | <state> | <task> | <effort h> | <corr> |
  <started_at> | <updated_at> | <checkpoint> | <done_by> | <evidence> |
```

## 3. Corrección (en CORRECTIONS.md)

```markdown
## CORR-<NNN> <A-<NNN>> — <síntoma>
- Fecha: <UTC>
- Síntoma: <observable>
- Causa raíz: <por qué>
- Fix aplicado: <paths>
- Prevención: <check/test/hook>
- Estado: fixed|parked|superseded
```

## 4. Handoff (al pausar un `started`)

```markdown
Pasado a: <role>/<ID>
Checkpoint: <último hito>
Próximo paso: <instrucción concreta>
Contadores: corr=<n>, effort=<h>
Evidencia parcial: <commits/hashes>
Bloqueador conocido: <sí/no + cuál>
```

## 5. Petición de bloqueo (estado `blocked`)

```markdown
Bloqueado en: A-<NNN>
Blocker: <descripción reproducible>
Necesito de: <role/recurso>
Alternativa ya probada: <qué fracasó>
Evidencia: <commits/hashes alcanzados>
```

## 6. Cierre (estado `done`)

```markdown
Cerrado: A-<NNN>
done_by: <role de grado >=2, ≠ autor>
Evidencia: <commit / digest / salida de check>
Correcciones: <n>
Siguiente dependencia (si aplica): A-<MMM> | -
```