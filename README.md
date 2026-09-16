# Diseño del entorno aislado

Instalador y componentes propios en Rust. Una cuenta Windows dedicada posee la distribución WSL; el usuario cotidiano accede únicamente a consultas autorizadas mediante una CLI.

Estado actual: propuesta de arquitectura, sin implementación del instalador ni validación del aislamiento en el host.

Consulta la [arquitectura con diagramas Mermaid](docs/arquitectura-wsl.md): identidades, canal de consulta, instalación y pruebas de aceptación.

## Criterios de trabajo

- Usar nombres funcionales neutrales; no heredar identificadores anteriores ni introducir marcas de proveedores en nombres propios.
- Mantener cada commit enfocado en un cambio relevante, de tamaño pequeño o mediano y fácil de revisar.
- Publicar la documentación compartida en `docs/`; mantener herramientas y notas locales en `.codex/` y `.agents/`, excluidos de Git.
