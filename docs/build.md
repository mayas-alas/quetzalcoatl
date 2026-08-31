# Build y verificación

## Artefactos

| Host objetivo | Salida | Contrato |
|---|---|---|
| Windows x86_64 | `dist/gnx-windows-x86_64.exe` | Instalador, CLI, Windows Service y tray. |
| Linux x86_64 | `dist/gnx-x86_64.AppImage` | Instalador y CLI distribuible. |
| Linux x86_64 | `dist/gnx-linux-x86_64` | ELF estático usado para empaquetado y diagnóstico. |

Los prerequisitos siguientes son sólo para construir. El usuario final abre el
EXE o AppImage y el instalador prepara el host automáticamente.

## Verificación común

Desde la raíz del repositorio:

```text
cargo fmt --check
cargo check --locked --all-targets
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

Rust está fijado en `rust-toolchain.toml`, crates en `Cargo.lock` y artefactos
externos en `dependencies.lock.toml`.

## Windows

Requiere Windows x86_64, Rust 1.98.0, MSVC Build Tools y Windows SDK:

```powershell
.\scripts\build-windows.ps1
```

El script ejecuta pruebas, construye release, copia el EXE, genera
`SHA256SUMS.windows` y verifica `version`. El PE enlaza el logo del instalador,
el icono de tray y metadata de producto desde `build.rs`.

```powershell
.\dist\gnx-windows-x86_64.exe --help
.\dist\gnx-windows-x86_64.exe --config .\config.example.toml status --json
Get-FileHash -Algorithm SHA256 .\dist\gnx-windows-x86_64.exe
```

No abra el EXE sin argumentos en el host de build salvo que quiera iniciar la
instalación real.

## Linux ELF

Desde Windows con una Podman Machine de build activa:

```powershell
.\scripts\build-linux.ps1
```

Desde Linux se puede ejecutar la misma lógica del script shell usada por el
empaquetado. El builder está fijado por digest y produce
`target/linux-musl/release/gnx` y `dist/gnx-linux-x86_64`.

## Linux AppImage

En Linux x86_64 con Podman, `sh`, `curl` o `wget`, `file`, `awk`, `sed` y
`sha256sum`:

```bash
sh scripts/build-appimage.sh
```

Si el ELF ya fue generado:

```bash
sh scripts/build-appimage.sh --skip-compile
```

El script usa el icono PNG oficial, verifica appimagetool y el runtime Type-2 por
SHA-256, y prueba el resultado sin depender de FUSE:

```bash
dist/gnx-x86_64.AppImage --appimage-extract-and-run version
sha256sum --check dist/SHA256SUMS.appimage
```

En un desktop Linux también debe probarse el montaje normal:

```bash
dist/gnx-x86_64.AppImage version
```

Desde Windows, después de `build-linux.ps1`:

```powershell
podman run --rm --arch amd64 `
  --volume "${PWD}:/workspace" `
  --workdir /workspace `
  docker.io/library/rust@sha256:3ffeca71d0e4fc30f5537f76b7243e87ac99726b6d3d66591dfc5e497078b9fc `
  sh -c 'apk add --no-cache file && sh scripts/build-appimage.sh --skip-compile'
```

## OpenTofu y Quadlets

OpenTofu se valida como módulo, pero en producción se ejecuta dentro del LXC
`gnx-infra-runner`, no en el host ni en la Podman Machine:

```bash
tofu -chdir=infra/opentofu fmt -check
TF_DATA_DIR=../../target/tofu-validate tofu -chdir=infra/opentofu init \
  -backend=false -input=false -lockfile=readonly
TF_DATA_DIR=../../target/tofu-validate tofu -chdir=infra/opentofu validate
bash -n guest/infra-runner-bootstrap.sh guest/infra-runner-run.sh guest/bootstrap.sh
```

Regeneración consciente del lock del provider:

```bash
bash scripts/lock-opentofu.sh
```

Validación de Quadlets sin arrancar Proxmox:

```bash
QUADLET_UNIT_DIRS="$PWD/runtime" /usr/libexec/podman/quadlet -dryrun
```

## Release local

Con los tres artefactos presentes:

```powershell
.\scripts\finalize-development-release.ps1
```

Genera `dist/SHA256SUMS` y `dist/release.json`.

## Acceptance física

Windows limpio:

1. Abrir el EXE y aceptar UAC.
2. Verificar instalación automática de WSL y Podman, incluido reboot/resume.
3. Abrir una shell nueva y ejecutar `gnx`, `gnx status` y `gnx doctor`.
4. Ejecutar `gnx logs`; verificar JSONL en `ProgramData`, servicio bajo
   `.\gnx-runtime`, tray inmediato y tray después de un logon.
5. Reiniciar y verificar recuperación de Podman Machine y unidades.

La shell que abrió el instalador conserva su entorno anterior; `PATH` se difunde
a Explorer para procesos nuevos, pero una shell ya abierta debe cerrarse y
abrirse. La ausencia de reinicio es correcta cuando WSL y el MSI no devuelven
`3010`/`1641`; el journal y `gnx logs` deben mostrar esa decisión.

Linux limpio:

1. Marcar el AppImage como ejecutable y abrirlo sin argumentos.
2. Aceptar `sudo`; no instalar prerequisitos manualmente.
3. Abrir una shell nueva y ejecutar `gnx`, `gnx status` y `gnx doctor`.
4. Verificar `gnx-host.service` tras reboot.
5. Verificar KVM, Proxmox, runner LXC, OpenTofu y workload LXC.

La desinstalación se prueba con `gnx uninstall`: Podman CLI debe desaparecer y
los datos de virtualización deben permanecer.
