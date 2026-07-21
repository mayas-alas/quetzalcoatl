# CI-03: restaurar validador autocontenido en runtime

Corrige únicamente la regresión de runtime en el worktree actual `codex/ci-dockur`. No hagas commit ni push.

Problema confirmado: el workflow copia `.github/scripts/validate-gnx-evidence.py` desde `$GITHUB_WORKSPACE`, pero este job deliberadamente no hace `actions/checkout`; el archivo no existe en Actions.

Resultado obligatorio:

- Elimina `.github/scripts/validate-gnx-evidence.py` del worktree.
- Materializa el validador completo y actual (sobre exacto, servicio exacto, binarios exactos, status recursivo, categorías mutuamente excluyentes y `--self-test`) mediante un heredoc **citado** directamente a `$RUNNER_TEMP/validate-gnx-evidence.py` dentro del paso de espera.
- Ejecuta `python3 "$validator" --self-test` antes del polling.
- No agregues `actions/checkout`, descargas raw, dependencias nuevas ni referencias a `$GITHUB_WORKSPACE`.
- Conserva toda la validación/allowlist implementada, la copia condicionada del artefacto, timeout y cleanup.
- Restaura los diagnósticos host seguros que existían antes del último cambio: `docker inspect` proyectado solo a Id/Image/Created/State, memoria/CPU/dispositivos/port bindings y Mount Type/Destination/RW (sin Env ni Source), `host-disk.txt` y `tailscale-serve.json`. El artefacto debe contener esos diagnósticos allowlisted más el sobre guest validado o el marcador fijo; no simplifiques eliminando evidencia útil.
- Para pruebas locales, extrae el heredoc real del workflow a un archivo temporal fuera del repo y ejecútalo; no mantengas un segundo source-of-truth rastreado.

Valida explícitamente:

- búsqueda sin resultados de `actions/checkout`, `GITHUB_WORKSPACE` y `.github/scripts/validate-gnx-evidence.py` en el workflow;
- YAML/actionlint, heredoc Bash válido, compilación Python y `--self-test` del código extraído;
- AST de bloques PowerShell, ShellCheck, fixtures BOM/parcial y marcador de artefacto;
- `git status` solo workflow+README, sin `.github/scripts`; `git diff --check`.

No cambies alcance y no hagas commit/push.
