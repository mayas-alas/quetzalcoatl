# Corrección CI-03: validador autocontenido y evidencia segura

Implementa únicamente la corrección de los hallazgos de revisión del diff no confirmado en el worktree actual `codex/ci-dockur`. No hagas commit ni push. No amplíes el producto ni agregues escenarios imaginarios.

Hallazgos que deben quedar resueltos:

1. El job no hace checkout y `$GITHUB_WORKSPACE/.github/scripts/validate-gnx-evidence.py` no existe en runtime.
2. La validación puede aceptar un servicio coincidente y cualquier binario aunque falte `gnx.exe`, y puede aceptar JSON inválido junto a un error arbitrario.
3. La evidencia subida conserva salida de estado sin una redacción estructural fiable; el `sed` actual no protege claves JSON entre comillas.

Diseño requerido, estrecho y verificable:

- Mantén el workflow autocontenido: materializa el validador mediante un heredoc citado en `$RUNNER_TEMP/validate-gnx-evidence.py` dentro del propio paso de espera. Elimina `.github/scripts/validate-gnx-evidence.py`; no añadas `actions/checkout`.
- Exige `installerSha256` exacto, un servicio GNX/Quetzalcoatl, y una entrada de `binaryHashes` cuyo `path` termine exactamente en `gnx.exe` (case-insensitive) con SHA-256 válido.
- Si existe `gnxStatusJson`, debe ser un objeto JSON válido con `schema_version == 1` y campos string no vacíos `overall` y `stage`. Un JSON inválido nunca pasa aunque también haya error.
- Solo permite ausencia de `gnxStatusJson` cuando `gnx.exe` sí fue encontrado y `gnxStatusError` sea una categoría fija producida por el colector para un comando que terminó con exit code no cero antes de la configuración interactiva. No guardes mensajes de excepción ni stdout/stderr arbitrarios.
- En `Collect-GnxEvidence.ps1`, captura la salida de `gnx status --json`, comprueba el exit code y parsea el JSON antes de almacenarlo. Persiste solo el esquema seguro conocido (`schema_version`, `overall`, `stage`, `role`, `controller`, `components`, `cluster`, `services`); omite o reemplaza `last_error` con un indicador fijo, nunca con texto libre.
- Para el artefacto host, no uses regex sobre JSON como frontera de seguridad. Copia únicamente la evidencia ya allowlisted por el colector y conserva diagnósticos host sin secretos; elimina la falsa afirmación/operación de redacción regex o añade una sanitización estructural que no exponga contenido. No imprimas el JSON ni razones con contenido de usuario en logs.
- Conserva el reintento seguro de archivos parciales/BOM, timeout fallido, guest detenido fallido, `if: always()` para artefactos y cleanup, límites 60/120/180, imagen Dockur fijada y distinción explícita frente a G5.
- Actualiza README solo si hace falta para describir la regla exacta.

Validaciones obligatorias:

- `actionlint`.
- Parseo AST de todos los bloques PowerShell embebidos.
- ShellCheck de todos los bloques Bash embebidos.
- Fixtures del validador materializado: BOM válido; falta de `gnx.exe`; hash incorrecto; servicio ausente; status JSON válido; status JSON inválido aun con error; categoría fija aceptada solo con `gnx.exe`; error arbitrario rechazado.
- `git diff --check`.

Entrega un resumen de archivos cambiados y resultados. No hagas commit ni push.
