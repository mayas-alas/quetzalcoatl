# Dependencias fijadas

Las versiones Rust exactas quedan en `Cargo.lock`; herramientas, imágenes y
payloads externos están en `dependencies.lock.toml`.

## Crates directos

| Crate | Responsabilidad |
|---|---|
| `clap` | Contrato CLI tipado y ayuda. |
| `serde`, `serde_json`, `toml` | Config, state, journal y reportes estrictos. |
| `url` | Canonicalización segura del controller. |
| `sha2` | Verificación streaming de artefactos. |
| `getrandom` | Secretos desde el CSPRNG del sistema. |
| `ureq` + platform verifier | HTTPS con trust store del host. |
| `windows-service` | Servicio Windows nativo. |
| `windows-sys` | UAC, filesystem, recursos y tray Win32. |
| `winresource` | Branding e iconos del PE durante el build. |

## Supply chain externa

| Componente | Fijación |
|---|---|
| Podman Windows | MSI 6.0.1, tamaño y SHA-256. |
| Tailscale | imagen amd64 1.102.3 por digest. |
| Docktail | imagen amd64 1.6.0 por digest. |
| Dockur/Proxmox | imagen amd64 observada 2026-08-29 por digest. |
| OpenTofu | tarball Linux amd64 1.12.6, tamaño y SHA-256. |
| BPG/Proxmox | provider 0.111.1 y lock con checksums firmados. |
| Ubuntu LXC | Noble 20260826 por URL inmutable y SHA-256. |
| AppImage | appimagetool 1.9.1 y runtime Type-2 20251108 por SHA-256. |

El builder Linux está fijado por digest. SBOM, auditoría de CVEs/licencias y
firmas de plataforma siguen siendo gates para un release público.
