# Corrección final CI-03: allowlist recursiva y basename exacto

Corrige únicamente los dos hallazgos de la revisión final en el worktree actual `codex/ci-dockur`. No hagas commit ni push.

Hallazgos:

1. El colector copia `components`, `cluster` y `services` completos desde `gnx status`, permitiendo propiedades anidadas arbitrarias en el artefacto.
2. El validador usa `endswith('gnx.exe')` y acepta `evilgnx.exe`.

Implementación requerida:

- Reconstruye `gnxStatusJson` recursivamente con el esquema exacto conocido:
  - top-level: `schema_version`, `overall`, `stage`, `role`, `controller`, `components`, `cluster`, `services`; `last_error` solo como indicador fijo `reported` cuando exista, nunca el texto.
  - `components`: únicamente `service`, `wsl`, `podman_machine`, `kvm`, `tailscale`, `tailscale_serve`, `proxmox`, `opentofu`, todos string.
  - `cluster`: únicamente `joined` y `quorate`, ambos boolean.
  - `services`: únicamente `garage` y `forgejo`, ambos string.
  - `role` y `controller`: string o null. Rechaza/no persistas el estado si los tipos requeridos no cumplen.
- Refuerza el validador Python embebido para verificar esa misma estructura, conjunto exacto de claves (permitiendo `last_error` solo con valor literal `reported`) y tipos. No permitas propiedades adicionales.
- Para `binaryHashes.path`, normaliza `\` a `/`, toma el último segmento y exige igualdad case-insensitive con `gnx.exe`; `evilgnx.exe` debe fallar.
- En el paso `if: always()` de artefactos, copia `gnx-evidence.json` solo si el validador embebido acepta el archivo. Si falta o es inválido, escribe únicamente un marcador fijo sin contenido de evidencia; nunca subas el JSON inválido/raw.
- Conserva hash exacto, servicio, categoría fija, BOM/archivo parcial, timeout, cleanup y separación G5.
- Ajusta README para decir basename exacto y artefacto validado estructuralmente.

Valida:

- `actionlint`.
- AST PowerShell de todos los bloques.
- ShellCheck de todos los Bash.
- Fixtures: status seguro válido; clave top-level extra; clave anidada extra; tipo anidado incorrecto; `last_error` libre; `evilgnx.exe`; basename exacto con slash y backslash; categoría fija; BOM/parcial.
- Demuestra que el paso de artefacto no copia evidencia inválida.
- `git diff --check`.

No amplíes alcance, no agregues dependencias y no hagas commit/push.
