# Quetzalcoatl GNX

Instalador y componentes propios en Rust. Una cuenta Windows dedicada posee la distribución WSL; el usuario cotidiano accede únicamente a consultas autorizadas mediante una CLI.

Estado actual: implementación experimental en Rust del instalador, servicio y CLI. El aislamiento y la instalación completa todavía requieren validación en una máquina de prueba.

Consulta la [arquitectura con diagramas Mermaid](docs/arquitectura-wsl.md): identidades, canal de consulta, instalación y pruebas de aceptación.

Consulta también la [entrega experimental](docs/entrega-experimental.md), sus comandos y limitaciones. Compilación: `cargo build --release --locked`. Pruebas: `cargo test --locked`.

El [naming fijo y la auditoría de doble revisión](docs/naming-auditoria.md) describen los identificadores de instalación, las correcciones comprobadas y los bloqueos pendientes antes de instalar.

## Criterios de trabajo

- Conservar la identidad **Quetzalcoatl GNX**: nombres compartidos en `src/lib.rs` e identificadores privados de instalación en `src/windows.rs`. Cambiarlos requiere considerar migraciones.
- Mantener cada commit enfocado en un cambio relevante, de tamaño pequeño o mediano y fácil de revisar.
- Publicar la documentación compartida en `docs/`; mantener herramientas y notas locales en `.codex/` y `.agents/`, excluidos de Git.

La base mantiene tres responsabilidades: entradas CLI/setup, contrato de consultas (`protocol.rs`) e integración Windows (`windows.rs`). Los helpers se extraen solo cuando eliminan duplicación o aseguran la liberación de recursos; no se añaden módulos para agrupar constantes.
