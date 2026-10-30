# Investigación Goose → GnX Core v0.0.19

Investigación del framework Goose (CLI, skills, browser integrado) y aterrizaje
para integrar las skills `.opencode/skills/` en el framework agentic de Quetzalcoatl.

## Estado de Goose en el host

- CLI: `C:\Program Files\Goose-win32-x64\dist-windows\resources\bin\goose.exe`
- Versión: `1.57.0`
- Skills locales: `C:\Users\mayas\.agents\skills\` (6 skills: `computer-use`,
  `find-skills`, `goose-doc-guide` builtin, `orca-cli`, `orchestration`)
- Configuración: `C:\Users\mayas\AppData\Roaming\Block\goose\config\config.yaml`

## Skills del repo vs Goose

- Las skills del repo (`.opencode/skills/`) no están en el directorio de Goose.
- `goose skills list` muestra solo las locales; `goose-doc-guide` es builtin.
- Para hacerlas aparecer: enlazar/copiar `.opencode/skills/` al directorio local
  o configurar Goose para leer `.opencode/skills/`. **Nota**: este mod aplica al
  usuario host de Windows (`mayas`), no al usuario dedicado `gnxcoresvc` ni al
  runtime WSL del servicio.

## Features de browser/navegador

- `orca-cli` opera el navegador integrado de Orca.
- `computer-use` opera ventanas visibles de apps locales.
- `goose.exe skills list` y `goose.exe info` confirman disponibilidad.

## Paradigma decidido: goose-driven

Todo trabajo del agente debe ser goose-driven: usar la CLI de Goose, sus skills,
sus extensiones (`computer-use`, `orca-cli`) y documentar cada paso. No usar
shelles arbitrarias sin pasar por Goose cuando sea posible.

## Abordaje para v0.0.19

1. Actualizar `Cargo.toml` a `0.0.19`.
2. Crear `releases/v0.0.19/CHANGELOG.md` con la integración.
3. Modificar `apps/gnx` (o agregar módulo) para automatizar la copia/enlace
   de skills y la ejecución de `goose skills list` como validación.
4. Documentar en `.AGENTS` o `.opencode/` que el framework usa Goose.
5. Compilar con `scripts/build.ps1` para generar `gnx.exe` versionado.

## Restricciones `.AGENTS`

- No crear tests sin autorización.
- No sobrescribir `releases/v0.0.18/`.
- Commit pequeño con `Hector <hector@email.gnx>`.
- Separar código, compilado, empaquetado y aplicado.
