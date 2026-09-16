# Quetzalcoatl GNX

Instalador y componentes propios en Rust. Una cuenta Windows dedicada posee la distribución WSL; el usuario cotidiano accede únicamente a consultas autorizadas mediante una CLI.

El [asistente gráfico y su reanudación tras reinicio](docs/asistente-instalacion.md) se abre ejecutando `quetzalcoatl-gnx-setup.exe` sin argumentos. Incluye progreso por fases, diagnóstico, UAC y reinicio consentido con cuenta atrás cancelable. Compilación/paquete Windows: `scripts/build-windows.ps1`.

La [instalación automática mediante WSL](docs/instalacion-automatica.md) permite `setup install <client-SID>` sin entregar un rootfs manualmente, con consulta de progreso mediante `quetzalcoatl-gnx wait`. La modalidad offline sigue disponible.

Estado actual: implementación experimental en Rust del instalador, servicio y CLI. El aislamiento y la instalación completa todavía requieren validación en una máquina de prueba.

Consulta la [arquitectura con diagramas Mermaid](docs/arquitectura-wsl.md): identidades, canal de consulta, instalación y pruebas de aceptación.

También está disponible el [diagrama Archify con vista previa y fuente editable](docs/diagrams/README.md).

Consulta también la [entrega experimental](docs/entrega-experimental.md), sus comandos y limitaciones. Para compilar, probar y empaquetar en Windows usa `scripts/build-windows.ps1` (incluye ajustes del linker para GNU y una prueba del EXE real).

El [naming fijo y la auditoría de doble revisión](docs/naming-auditoria.md) describen los identificadores de instalación, las correcciones comprobadas y los bloqueos pendientes antes de instalar.

## Criterios de trabajo

- Conservar la identidad **Quetzalcoatl GNX**: nombres compartidos en `src/lib.rs` e identificadores privados de instalación en `src/windows.rs` y `src/windows/installer.rs`. Cambiarlos requiere considerar migraciones.
- Mantener cada commit enfocado en un cambio relevante, de tamaño pequeño o mediano y fácil de revisar.
- Publicar la documentación compartida en `docs/`; mantener herramientas y notas locales en `.codex/` y `.agents/`, excluidos de Git.

La base separa entradas CLI/setup, contrato de consultas (`protocol.rs`), integración Windows (`windows.rs`) y el asistente reanudable (`installer.rs`, `windows/installer.rs`, `setup_gui.rs`). Los helpers se extraen solo cuando eliminan duplicación o aseguran la liberación de recursos; no se añaden módulos para agrupar constantes.
