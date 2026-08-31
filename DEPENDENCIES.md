# Dependencias fijadas

Las versiones Rust exactas quedan en `Cargo.lock`; las herramientas, imágenes y
payloads externos están en `dependencies.lock.toml`.

## Crates directos

| Crate | Responsabilidad |
|---|---|
| `clap` | Contrato CLI tipado y help. |
| `serde`, `serde_json`, `toml` | Config, state, journal y reportes estrictos. |
| `url` | Canonicalización segura del controller. |
| `sha2` | Verificación streaming de artefactos. |
| `getrandom` | Secretos generados desde el CSPRNG del sistema. |
| `ureq` + platform verifier | HTTPS bloqueante con trust store del host. |
| `windows-service` | Registro y ejecución nativos del servicio Windows. |
| `windows-sys` | UAC y reemplazo/limpieza atómicos de archivos Windows. |

## Supply chain externa

| Componente | Fijación |
|---|---|
| Podman Windows | MSI 6.0.1, tamaño y SHA-256. |
| Tailscale | imagen amd64 1.102.3 por digest. |
| Docktail | imagen amd64 1.6.0 por digest. |
| Dockur/Proxmox | imagen amd64 observada 2026-08-29 por digest. |
| OpenTofu | tarball Linux amd64 1.12.6, tamaño y SHA-256. |
| BPG/Proxmox provider | 0.111.1 y checksums firmados en `.terraform.lock.hcl`. |
| Ubuntu LXC | Noble 20260826 por URL inmutable y SHA-256. |
| AppImage | appimagetool 1.9.1 y runtime Type-2 20251108 por SHA-256. |

El builder Linux es `rust:1.98.0-alpine` por digest. Antes de un release público
faltan SBOM, auditoría transitiva de CVEs/licencias y firmas de plataforma.
