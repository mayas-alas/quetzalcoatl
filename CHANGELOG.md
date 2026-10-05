# Changelog

Los cambios pendientes no equivalen a una release publicada. Cada release conserva sus notas de validación y limitaciones; los artefactos publicados son inmutables.

## [Unreleased]

### Added
- Candidato Proxmox propio reconstruido desde Debian, sin instalar `pve-fake-subscription`; runtime upstream versionado y licencia MIT conservada.
- Pipeline externo Podman/Skopeo: build, inventario, OCI exportado, inspección y copia con digest preservado; promoción explícita al repositorio OCI e importer para el almacén del nodo.

### Security
- Se descarta el canal de descarga HTTP de archivos OCI desde el runtime. Sin credenciales en argv/env, TLS desactivado ni referencias de producción sin digest.

### Pending
- Configurar el registry/namespace autorizado y publicar el candidato; no se inventa un destino ni se cambia el Quadlet antes de verificar la referencia remota.
- Integrar la adquisición antes del arranque/Retry, añadir Skopeo al nodo, publicar un instalador nuevo y validar upgrade live con VMs/LXC. v0.1.53 y compute en ejecución permanecen intactos.

## [0.1.53] — 2026-10-05

[Notas y límites de validación](releases/v0.1.53/RELEASE_NOTES.md) · [Hashes](releases/v0.1.53/SHA256SUMS.txt).

### Changed
- GitHub en la PWA: una operación directa con token clásico (`repo`, `workflow`), confirmación, creación de repositorio privado y carga del YAML elegido. Sin puente Windows, UAC, OAuth ni controles de ejecución de Actions.
- `releases/` conserva únicamente la última release validada. Las notas Markdown históricas se archivan en `docs/releases/`.

### Changed — distribución
- La publicación v0.1.53 entrega Burn con el MSI embebido y hashes; no duplica el MSI separado, para respetar la cuota de 1 GB de Hugging Face sin purgar historial. El build y la verificación siguen comprobando ambos outputs.

### Changed — licencia
- AGPLv3 únicamente (`AGPL-3.0-only`), texto completo en `LICENSE`, metadata del workspace y Hugging Face, archivo incluido en MSI y enlace en Burn. Las dependencias conservan sus licencias originales.

### Security
- Token GitHub solo en memoria durante la operación; sin persistencia, logs, URLs ni caché. Peticiones exclusivamente a `https://api.github.com`, sin redirecciones ni cookies.
- No se sobrescriben repositorios/workflows existentes ni se ejecuta Actions automáticamente. Los secrets del workflow se configuran en GitHub.

## [0.1.52]

- [Notas y validación](docs/releases/v0.1.52/RELEASE_NOTES.md).
- Instaladores retirados del árbol actual; no se modifica el historial Git/LFS.

## Histórico

- [0.1.21](docs/releases/v0.1.21/RELEASE_NOTES.md)
- [0.1.10](docs/releases/v0.1.10/RELEASE_NOTES.md)
- [0.1.8](docs/releases/v0.1.8/RELEASE_NOTES.md)
- [0.1.7](docs/releases/v0.1.7/RELEASE_NOTES.md)
- [0.1.6](docs/releases/v0.1.6/RELEASE_NOTES.md)
- [0.1.5](docs/releases/v0.1.5/RELEASE_NOTES.md)
- [0.1.4](docs/releases/v0.1.4/RELEASE_NOTES.md)
- [0.1.3](docs/releases/v0.1.3/RELEASE_NOTES.md) — [evidencia](docs/releases/v0.1.3/UPGRADE_EVIDENCE.md)
- [0.1.2](docs/releases/v0.1.2/RELEASE_NOTES.md)
- [0.1.1](docs/releases/v0.1.1/RELEASE_NOTES.md)

## Política de publicación

1. Registrar cambios en `Unreleased` usando `Added`, `Changed`, `Fixed`, `Removed` o `Security` según corresponda.
2. Antes de publicar, crear las notas de la nueva versión con pruebas realmente ejecutadas y límites de aceptación. Actualizar este índice; no afirmar validación live sin evidencia.
3. Ejecutar build y verificación canónicos. `installer/scripts/verify.ps1 -PublishRelease` conserva artefactos solo de la versión nueva y archiva el Markdown anterior.
4. Publicar desde la WSL autenticada del usuario host, tras reconciliar el remoto sin force-push.
