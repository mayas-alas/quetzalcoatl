# Hardening final CI-03: sobre exacto e identidad de servicio

Corrige únicamente los tres hallazgos de la revisión de release en `codex/ci-dockur`. No hagas commit ni push.

Implementación requerida:

1. Colector guest:
   - Obtén solo el servicio cuyo `Name` sea exactamente `Quetzalcoatl` (case-insensitive), no coincidencias por substring/ruta.
   - Emite entradas de servicio con claves exactas `name`, `state`, `startMode`, `startName`; no incluyas otros campos.
   - Mantén entradas de binarios con claves exactas `path`, `sha256`.
   - Solo agrega `safeStatus.last_error = 'reported'` si el valor original `status.last_error` es realmente no-null; propiedad presente con null debe omitirse.

2. Validador host:
   - Exige claves top-level exactas: `schemaVersion`, `collectedAtUtc`, `installerSha256`, `services`, `binaryHashes`, `gnxStatusJson`, `gnxStatusError`.
   - `schemaVersion` debe ser entero 1 (no bool); `collectedAtUtc` string no vacío; hash exacto.
   - Exige exactamente una entrada de servicio, claves exactas, todos sus valores string, `name == Quetzalcoatl` y `startName == NT SERVICE\\Quetzalcoatl` case-insensitive. No aceptes substrings. Mantén `state`/`startMode` como evidencia tipada sin inventar estados.
   - Todas las entradas de `binaryHashes` deben tener exactamente `path`/`sha256`, ambos string y SHA válido; al menos una debe tener basename exacto `gnx.exe`.
   - Si hay status JSON válido, `gnxStatusError` debe ser null. Si no hay status, `gnxStatusJson` debe ser null y `gnxStatusError` debe ser la categoría fija. Rechaza combinaciones ambiguas.
   - Conserva la validación recursiva exacta del status.
   - Con estas reglas, el paso de artefacto solo puede copiar el sobre allowlisted; en cualquier fallo conserva únicamente el marcador fijo.

3. README: documenta identidad exacta de servicio y sobre estructuralmente validado, sin afirmar G5.

Fixtures obligatorias adicionales:

- clave top-level extra;
- clave extra/tipo incorrecto en cada entrada de servicio/binario;
- servicio `fake-quetzalcoatl` y cuenta incorrecta;
- más de un servicio;
- combinación status+error simultánea;
- `last_error:null` se omite y `last_error` no-null se transforma en `reported`;
- válidos por status y por categoría fija;
- inválido nunca se copia al artefacto.

Repite actionlint, AST PowerShell, ShellCheck, BOM/parcial y `git diff --check`. No amplíes alcance y no hagas commit/push.
