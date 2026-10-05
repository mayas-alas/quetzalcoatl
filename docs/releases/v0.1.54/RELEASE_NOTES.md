# GnX Node v0.1.54

## Integración de compute

- Fuente de imagen: https://github.com/mayas-alas/tailnet-proxmox, commit `bbe04d38b4b480fcce9038eb994eb0b81bd7b474`.
- Upstream `dockur/proxmox` actualizado a `5fccb4a7`; único cambio funcional del Dockerfile: quitar la instalación de `pve-fake-subscription`. Se retiran las personalizaciones antiguas del repo de imagen.
- Referencia del producto: `ghcr.io/mayas-alas/tailnet-proxmox@sha256:bc48f533f0daddb045128caf052b6b0a15d045cde7106f60ed3d906798adc0b9` (`9.2.21-gnx.1`, linux/amd64).
- Podman/Quadlet gestiona la descarga con `Pull=missing`. Install/Retry adquiere la imagen por digest antes de reemplazar el Quadlet, con retries compatibles con Podman 4.9. Si falla la descarga, no se cambia ni detiene compute.
- Quetzalcoatl no incluye Containerfile, código upstream de la imagen, builders, Skopeo, promoción ni importer HTTP. La construcción y publicación de imágenes vive en GitHub.
- Se conservan volúmenes compute/config, identidad, WSL, CA, credenciales, gateway y reglas de red del producto. UpgradeCodes estables y versión MSI/Burn `0.1.54.0`.

## Validación — 2026-10-05

- CI de imagen [37352060719](https://github.com/mayas-alas/tailnet-proxmox/actions/runs/37352060719): build, ausencia del paquete falso, boot aislado de Proxmox, pveproxy/pvedaemon/pvestatd, HTTPS y KVM aprobados; publicación Skopeo con digest preservado y comprobado.
- Descarga anónima de la referencia exacta desde GHCR con Podman **4.9.3** en la WSL del usuario host: aprobada. Paquetes: `proxmox-ve 9.2.0`, `pve-manager 9.2.21`, `qemu-server 9.2.10`, `pve-container 6.1.14`.
- `cargo test --workspace --locked`: 62 pruebas aprobadas. Warning de linker debug LNK4098; no fallos de tests.
- Build Rust release, MSI y Burn: aprobado, sin errores ni warnings WiX.
- Verificación de rootfs sellado, ProductVersion MSI, hashes de artefactos y contratos estáticos installer/runtime/LXC/red/PWA/workload: aprobada.
- Payload empaquetado: referencia GHCR y prefetch presentes; fuentes/builders de imagen ausentes.
- Prefetch empaquetado probado en memoria: caché sin descarga, adquisición correcta y fallo bloqueante antes del reemplazo; sin fixtures escritos.

No se afirma upgrade live de v0.1.54 sobre las VMs/LXC existentes ni alta en host limpio. No se ha instalado este bundle, reiniciado compute ni probado migración/downgrade de datos en el host activo.

## Descarga y licencia

[Descargar GnxNodeSetup.exe](https://huggingface.co/datasets/gnx-labs-mx/gnx-node-releases/resolve/1a2c95961464503a2bfbeec8fdd1a090ee00a852/releases/v0.1.54/GnxNodeSetup.exe) — revisión de artefacto inmutable, SHA-256 remoto verificado.

El instalador completo `GnxNodeSetup.exe` contiene el MSI construido; no se duplica el MSI separado. AGPL-3.0-only, texto incluido en el paquete. Dependencias e imagen conservan sus licencias originales.

Los instaladores se publican en el repositorio de artefactos Hugging Face `gnx-labs-mx/gnx-node-releases`. El Space Quetzalcoatl conserva fuente, notas, hashes y enlace de descarga: así se respeta su cuota de 1 GB sin purgar historial ni modificar el runtime. `releases/` conserva únicamente esta versión y las notas históricas pasan a `docs/releases/`.

Verifica el instalador contra `SHA256SUMS.txt` antes de ejecutarlo.
