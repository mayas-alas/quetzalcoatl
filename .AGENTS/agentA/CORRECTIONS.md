# CORRECTIONS — registro de errores

Cada retrabajo se documenta aquí **antes** de reintentar. Plano y git-friendly;
el campo `corr` de `TRACKING.md` debe cuadrar con los renglones por `ID`.

## Cómo registrar una corrección

```markdown
## CORR-<NNN> <ID ticket> — <síntoma breve>
- Fecha: YYYY-MM-DD UTC
- Síntoma: qué falló observable.
- Causa raíz: por qué pasó (no el síntoma).
- Fix aplicado: qué se cambió y dónde (paths).
- Prevención: qué evita que se repita (check/test/hook).
- Estado: fixed | parked | superseded
```

Las correcciones nunca se borran; una resuelta se marca `fixed`, no se elimina.

## Registro

_No hay correcciones registradas (primer consolidado del marco `agentA`)._

## Tono

- Unicode ok; evita tablas dentro de fenced code (se romperá el diff).
- Si una corrección toca algo que otros role reutilizan, deja en `Prevención`
  la referencia a la sección de `FLOW.md`/`GAMIFIED.md` que la exige.