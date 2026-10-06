# Changelog

Los cambios pendientes no equivalen a una release publicada. Cada release conserva sus notas de validación y limitaciones; los artefactos publicados son inmutables.

## [Unreleased]

Sin cambios pendientes de release.

## [0.1.72] — 2026-10-06

[Notas](releases/v0.1.72/RELEASE_NOTES.md) · [Artefacto y hash](releases/v0.1.72/ARTIFACT.md) · [Evidencia](docs/validation/v0.1.72/README.md).

- Corrige el 500 del callback Pocket con `email_verified=false`: identidad por
  `sub` firmado y acceso por grupo; sin bypass de firma, nonce, CSRF o grupos.
- Fix empaquetado en Burn/MSI offline; upgrade exacto instalado exit 0,
  `gnx 0.1.72`, NodeReady y 53 checks runtime + payload actual pass.
- SID, CA, credenciales Windows/OIDC/RustFS/Pocket y directorios persistentes
  preservados; compute no recreado. Login personal observado con el hotfix.
- 66 tests Rust, 16 checks OIDC aislados y 6 escenarios del MSI compilado pass.
  El upgrade real omite cleanup destructivo en MSI viejo/nuevo.
- No se ejecutó desinstalación destructiva ni se prueban host limpio/cold boot,
  peers reales o logout/revocación Pocket personal completos.

## [0.1.71] — candidato histórico

- Candidato v0.1.71: OAuth2 Proxy/Pocket ID/Valkey protegen la PWA completa,
  grupos explícitos y logout local con revocación del servidor. Sin migración
  a NetBird, personas inventadas ni asignación automática de membresías.
- Máquina compartida denominada servicios privados en árbol/PWA/CLI; conserva
  claves, tag, datastore y URI `rustfs` como compatibilidad.
- Podman 4.9 usa `PodmanArgs` para host Pocket; reconciliación actualiza mounts de
  gateway; retiro del worker no vuelve a registrarse ni genera bucle de recarga.
- 66 tests Rust, controles de PWA/worker, imágenes offline y OIDC aislado. La
  aceptación con passkey Pocket personal continúa pendiente; ver [guía](docs/pwa-access.md).

Pendiente aceptación personal con passkey y tráfico entre peers reales; no equivale a fallo de disponibilidad de los servicios.

## [0.1.67] — 2026-10-05

[Notas y límites](docs/releases/v0.1.67/RELEASE_NOTES.md). [Evidencia del nodo instalado](docs/validation/v0.1.67/README.md).

- Pocket ID + NetBird combinado + dashboard separados, offline y privados, en el LXC existente.
- OIDC asociado; gRPC h2c, relay WebSocket y STUN comprobados por el gateway real.
- Upgrade instalado alcanza `NodeReady`, conservando credenciales; PWA y Firefox muestran servicios operativos y el flujo NetBird → Pocket ID.
- Preflight ACL bidireccional, STUN forwarding estrecho, readiness protocolario y errores/diagnóstico sanitizados.
- EXE e input offline en dataset; fuente/notas/evidencia en Space. Sin purga de historial.
- Login personal completo y tráfico de peers **no demostrados**; candidatos fallidos v0.1.60–v0.1.66 no promovidos.

## [0.1.59] — 2026-10-05

[Notas históricas](docs/releases/v0.1.59/RELEASE_NOTES.md). Entrega previa incompleta: dashboard sin backend combinado; no demuestra login browser ni peers operativos.

- Quadlets en el LXC gestionado para `pocket-id` y `netbird-dashboard`, con imágenes incluidas en el bundle offline, `Pull=never`, volúmenes persistentes y sin publicar puertos underlay.
- Accesos privados nuevos `https://pocket.gnx` y `https://netbird.gnx` por Caddy/TLS del nodo, DNS split y hosts loopback; backends sólo por mesh (`*.mesh.gnx`).
- Pocket ID genera `ENCRYPTION_KEY` persistente y NetBird dashboard queda parametrizado contra Pocket ID como autoridad OIDC.

## [0.1.58] — 2026-10-05

[Notas históricas](docs/releases/v0.1.58/RELEASE_NOTES.md).

- Candidato con Quadlets y rutas iniciales; reemplazado por v0.1.59 para incluir las imágenes offline y la validación browser completa sin depender de descarga runtime.

## [0.1.57] — 2026-10-05

[Notas históricas](docs/releases/v0.1.57/RELEASE_NOTES.md).

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
