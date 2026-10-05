# Changelog

Los cambios pendientes no equivalen a una release publicada. Cada release conserva sus notas de validación y limitaciones; los artefactos publicados son inmutables.

## [Unreleased]

Sin cambios pendientes.

## [0.1.57] — 2026-10-05

[Notas y validación](releases/v0.1.57/RELEASE_NOTES.md) · [Hashes](releases/v0.1.57/SHA256SUMS.txt).

- Status no entra en un distro manualmente detenido: primero inspecciona `wsl --list --running` bajo la identidad dedicada y proyecta reconciliación pendiente. Así no arranca compute por consultar el checkpoint.
- Misma imagen limpia `9.2.21-gnx.2`; no necesita recreación al aplicar este cambio sólo de host.

## [0.1.56] — 2026-10-05

[Notas históricas](docs/releases/v0.1.56/RELEASE_NOTES.md).

### Fixed
- MSI instalado ya no se confunde con runtime aplicado: snapshot/PWA reportan `RUNTIME_UPDATE_PENDING`; Setup pide confirmación y `gnx upgrade --confirm` aplica conservando enrolamiento.
- Lifecycle externo compara imagen activa y configuración reconocida, no sólo caché/Quadlet. Descarga previa al reemplazo y receipt sólo tras activación verificada; Retry no adopta un contenedor viejo tras un fallo.
- Readiness exige release correcto, ausencia del paquete falso y del bloqueo de licencias, además de KVM/HTTPS/workloads. Runner WSL 15 minutos; diagnóstico sin materialización ni cambios.
- Password rotation sigue recreando compute para consumir su EnvironmentFile protegido.
- La aceptación tras activar espera hasta 5 minutos al verificador canónico para el warm-up de DHCP/mesh/gateway del LXC, sin reiniciar ni re-enrollar por un transitorio. El candidato local v0.1.55 detectó esta carrera y no se publicó.

### Removed
- `blockLicense()` upstream y su llamada, que añadían overrides de `shop.maurer-it.com` independientemente del `.deb`. Imagen `9.2.21-gnx.2`, publicada/probada en GitHub CI 37360545004; sin builder/OCI importer en el producto.

### Distribución
- Bundle completo y hashes en la carpeta última release del Space Hugging Face; no duplicación del MSI ni purga de historial. Los inputs sellados del build se conservan para reproducibilidad.

## [0.1.54] — 2026-10-05

[Notas y límites de validación](docs/releases/v0.1.54/RELEASE_NOTES.md). Historial preservado; carpeta anterior retirada del árbol actual.

### Changed
- Compute consume `ghcr.io/mayas-alas/tailnet-proxmox:9.2.21-gnx.1` fijado por digest en el Quadlet; `Pull=missing` deja la descarga/almacén en Podman, sin credenciales nuevas.
- Imagen mantenida en [tailnet-proxmox](https://github.com/mayas-alas/tailnet-proxmox), actualizada desde `dockur/proxmox` (`5fccb4a7`): único cambio funcional del Dockerfile, quitar la instalación de `pve-fake-subscription`. Scripts upstream sin reglas antiguas personalizadas.
- CI de imagen [37352060719](https://github.com/mayas-alas/tailnet-proxmox/actions/runs/37352060719): build amd64, ausencia del paquete, boot de Proxmox, pveproxy/pvedaemon/pvestatd, HTTPS y KVM aprobados antes de publicación con Skopeo. Digest remoto verificado y lectura anónima confirmada.

### Removed
- Containerfile, código upstream vendorizado, builders, promoción e importer de imágenes del workspace Quetzalcoatl. No se añade Skopeo al nodo ni un downloader HTTP de OCI.

- Install/Retry descarga primero la imagen fijada por digest y solo después reemplaza el Quadlet; retries compatibles con Podman 4.9.
- Instalador v0.1.54 compilado/verificado. Binarios en `gnx-labs-mx/gnx-node-releases` (Hugging Face); el Space conserva fuente, notas, hashes y enlace sin duplicar ~310 MB por versión.

### Límites de aceptación
- Upgrade sobre VMs/LXC existentes y alta en host limpio pendientes; no se modifica el host activo. Los smoke tests aislados no prueban migración de datos.

## [0.1.53] — 2026-10-05

[Notas y límites de validación](docs/releases/v0.1.53/RELEASE_NOTES.md). Instalador anterior retirado del árbol actual; historial Git/LFS preservado.

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
