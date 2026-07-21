# Revisión final independiente CI-03

Actúa solo como revisor del diff no confirmado de `codex/ci-dockur`. No edites, no hagas commit ni push.

Devuelve hallazgos P1/P2/P3 con ruta y línea. Si no hay defecto accionable, responde exactamente `CLEAN` y una verificación breve.

Revisa estrictamente `.github/workflows/windows-rdp-tailscale.yml` y `README.md`:

1. El validador embebido debe existir en runtime sin checkout y el heredoc debe ser válido bajo YAML/Bash.
2. Solo verde con hash exacto del instalador, servicio GNX/Quetzalcoatl, **basename exacto** `gnx.exe` (no `evilgnx.exe`) y SHA-256 válido.
3. JSON de estado: objeto, schema 1, `overall`/`stage` strings; JSON inválido jamás pasa aunque haya error. La única alternativa debe ser la categoría fija, solo con `gnx.exe` presente.
4. El colector no debe persistir stdout/stderr/excepciones arbitrarias ni propiedades fuera del esquema allowlisted; revisa especialmente `last_error`, objetos anidados y archivos parciales/BOM.
5. Ningún secreto puede aparecer en logs, summary o artefactos; `docker inspect` no debe incluir env/password. La evidencia subida en caminos de fallo debe seguir siendo segura.
6. Timeout, guest detenido, archivo ausente/parcial deben fallar; `if: always()` debe preservar artefactos y cleanup.
7. No se afirma G5/quorum/convergencia y no se automatizan secretos interactivos.

Comprueba las validaciones declaradas si es útil, incluida extracción/ejecución del validador real. No amplíes el alcance.
