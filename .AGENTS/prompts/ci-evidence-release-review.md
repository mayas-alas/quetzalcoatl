# Revisión de release CI-03

Actúa solo como revisor. No edites, no commit, no push. Devuelve P1/P2/P3 con ruta/línea o exactamente `CLEAN` con verificación breve.

Revisa el diff completo de `.github/workflows/windows-rdp-tailscale.yml` y `README.md` como frontera de seguridad/aceptación final.

Además de los invariantes previos, comprueba explícitamente:

- El reporte top-level que puede copiarse al artefacto debe tener una allowlist exacta; `services` y `binaryHashes` también deben limitar claves/tipos para que un JSON manipulado no pueda introducir secretos y aun pasar.
- El servicio aceptado debe corresponder al producto instalado, no a una coincidencia arbitraria evitable.
- `last_error` solo debe producir el indicador `reported` cuando el valor original sea no-null; un `last_error: null` no debe presentarse como error reportado.
- La validación PowerShell de arrays/pipelines y tipos debe comportarse como se pretende en Windows PowerShell 5.1.
- El validador host debe reproducir exactamente el esquema emitido; un archivo inválido nunca se copia al artefacto.
- Basename exacto, hash exacto, BOM/parcial, fixed error, timeout/fallo, always cleanup, no secretos/logs y distinción G5.

Usa validaciones read-only si ayudan. No agregues escenarios fuera del PoC/MVP.
