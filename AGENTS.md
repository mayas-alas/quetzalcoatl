# Agentes — GnX Core

Lee y cumple [`.AGENTS`](.AGENTS) antes de actuar. Es el contrato operativo común;
[scope.md](scope.md) fija el alcance y [architecture.md](architecture.md) describe
la implementación. Este archivo orienta; las skills aportan procedimientos, no permisos.

## Selección de skills

Carga solo las necesarias, por ID con la herramienta `skill` o mediante `@id` en
OpenCode. Disponible no significa cargada; una skill no instala herramientas ni
demuestra que una sesión OAuth o un modelo estén operativos.

| Custom | Señal de activación / salida |
| --- | --- |
| [`gnx-vector`](.opencode/skills/gnx-vector/SKILL.md) | Intención difusa o desvío: objetivo, exclusiones y señal de cierre. |
| [`gnx-friccion`](.opencode/skills/gnx-friccion/SKILL.md) | Supuesto decisivo sin sostén: evidencia que podría cambiar la decisión. |
| [`gnx-umbral`](.opencode/skills/gnx-umbral/SKILL.md) | Cambio de riesgo o afirmación de éxito: avanzar, acotar o detener con fundamento. |

No son roles ni fases obligatorias. Activa una por una necesidad concreta y
ciérrala cuando produzca su salida; sus enlaces no obligan a cargar las demás.
Una tarea directa no necesita ceremonia. Cualquier agente puede usar estas lentes.

| Skill existente | Cuándo usarla / ejemplo |
| --- | --- |
| `orchestration` | Ejecutar coordinación autorizada: workers, mensajes, dependencias y bloqueos. |
| `orca-cli` | Worktrees, terminales, handoffs y navegador integrado de Orca: «Lee esa terminal». |
| `computer-use` | Ventanas visibles que CLI/API no puedan operar: «Inspecciona ese diálogo». No sustituye al navegador de Orca. |
| `opencode` | Configuración, modelos, permisos, MCP y diagnóstico de OpenCode: «Revisa la configuración». |
| `find-skills` | Investigar nuevas skills: «Busca una skill de Rust». No autoriza instalar ni actualizar. |
| `report` | Preparar un reporte de error de OpenCode; publicar requiere confirmación. |

Las customs deciden qué falta; las existentes aportan el procedimiento operativo.
Prefiere filesystem/Git/CLI/API cuando basten. Comprueba disponibilidad y permisos
antes de enlazar una herramienta; no sustituyas una denegación por shell o GUI.

## Persistencia

El marco propio vive aquí, en `.AGENTS` y en `.opencode/`; no se parchean skills
globales ni integradas. `.opencode/opencode.json` carga el contrato y limita los
IDs permitidos. Una skill nueva requiere revisar contenido, compatibilidad y
permisos antes de incorporarla. Consulta las guías nativas vigentes al usarlas,
sin copiar su CLI en las customs. No cargues todo el catálogo por rutina.
