# Build y verificación

## Salidas

| Plataforma | Artefacto | Uso |
|---|---|---|
| Windows x86_64 | `dist/gnx-windows-x86_64.exe` | Instalador inicial, CLI y servicio. |
| Linux x86_64 | `dist/gnx-x86_64.AppImage` | Instalador inicial y CLI distribuible. |
| Linux x86_64 | `dist/gnx-linux-x86_64` | ELF musl de desarrollo. |

El usuario final no necesita preparar WSL, Podman CLI o QEMU. Los requisitos de
esta guía son sólo para construir los artefactos.

## Validaciones comunes

Desde la raíz del proyecto:

```text
cargo fmt --check
cargo check --locked --all-targets
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

Rust queda fijado en `rust-toolchain.toml`, crates en `Cargo.lock` y payloads
externos en `dependencies.lock.toml`.

## Windows EXE

Requisitos de build: Windows x86_64, Rust 1.98.0 y Build Tools MSVC.

```powershell
.\scripts\build-windows.ps1
```

El script prueba el crate, compila release, copia el EXE, calcula SHA-256 y ejecuta
`version` sobre el artefacto. Verificación segura adicional:

```powershell
.\dist\gnx-windows-x86_64.exe --help
.\dist\gnx-windows-x86_64.exe --config .\config.example.toml status --json
Get-FileHash -Algorithm SHA256 .\dist\gnx-windows-x86_64.exe
```

No abra el EXE sin argumentos en la máquina de build salvo que quiera instalar:
ese es deliberadamente el recorrido del usuario final.

## Linux ELF desde Windows

Con Podman Machine activa en el host de build:

```powershell
.\scripts\build-linux.ps1
```

El script ejecuta tests y build dentro de la imagen Rust/Alpine fijada por digest,
y verifica el ELF dentro de Linux.

## AppImage desde Linux

Requisitos de build: Linux x86_64, Podman, `bash`, `curl`, `awk`, `sed` y
`sha256sum`.

```bash
bash scripts/build-appimage.sh
```

El script construye el ELF, crea AppDir, verifica appimagetool y el runtime Type-2
por SHA-256, empaqueta y ejecuta:

```bash
dist/gnx-x86_64.AppImage --appimage-extract-and-run version
sha256sum --check dist/SHA256SUMS.appimage
```

Si el ELF ya existe:

```bash
bash scripts/build-appimage.sh --skip-compile
```

En un desktop con FUSE también debe pasar:

```bash
dist/gnx-x86_64.AppImage version
```

## AppImage desde Windows

Después de `build-linux.ps1`:

```powershell
podman run --rm --arch amd64 `
  --volume "${PWD}:/workspace" `
  --workdir /workspace `
  docker.io/library/rust@sha256:3ffeca71d0e4fc30f5537f76b7243e87ac99726b6d3d66591dfc5e497078b9fc `
  bash scripts/build-appimage.sh --skip-compile
```

## OpenTofu/provider lock

Para regenerar conscientemente `.terraform.lock.hcl` después de cambiar una
versión fijada:

```bash
bash scripts/lock-opentofu.sh
```

Validación del módulo:

```bash
tofu -chdir=infra/opentofu fmt -check
TF_DATA_DIR=../../target/tofu-validate tofu -chdir=infra/opentofu init \
  -backend=false -input=false -lockfile=readonly
TF_DATA_DIR=../../target/tofu-validate tofu -chdir=infra/opentofu validate
```

Los Quadlets se pueden validar sin arrancar Proxmox:

```bash
QUADLET_UNIT_DIRS="$PWD/runtime" /usr/libexec/podman/quadlet -dryrun
```

## Metadata conjunta

Con los tres artefactos presentes:

```powershell
.\scripts\finalize-development-release.ps1
```

Genera `dist/SHA256SUMS` y `dist/release.json`.

## Acceptance de instalación

### Windows limpio

1. Abrir `gnx-windows-x86_64.exe`.
2. Aceptar UAC.
3. Confirmar descarga verificada de Podman y preparación de WSL.
4. Reiniciar si se solicita y confirmar reanudación.
5. Abrir una shell nueva y ejecutar `gnx`, `gnx status` y `gnx doctor`.
6. Confirmar servicio `QuetzalcoatlNext` con cuenta
   `NT SERVICE\QuetzalcoatlNext`.
7. Apagar/encender y confirmar que la máquina y unidades vuelven sin intervención.

### Linux limpio

1. Marcar `gnx-x86_64.AppImage` como ejecutable y abrirlo sin argumentos.
2. Aceptar `sudo`.
3. Abrir una shell nueva y ejecutar `gnx`, `gnx status` y `gnx doctor`.
4. Confirmar `gnx-host.service` habilitado.
5. Reiniciar y confirmar Podman Machine, Proxmox, OpenTofu y LXC.

La validación completa exige KVM/nested virtualization. Un host de build sin KVM
puede validar código, HCL, Quadlets y empaquetado, pero no cerrar `KVM-01/LXC-01`.

## Mantenimiento

Una actualización consume un artefacto local y checksum del release:

```text
gnx update --from <EXE-o-AppImage> --sha256 <SHA-256>
```

La desinstalación es explícita:

```text
gnx uninstall
```

Retira GNX, servicio, `PATH` y Podman CLI. Conserva configuración y todos los
datos de virtualización.
