# Arquitectura incremental funcional

```text
quetzalcoatl-gnx/
├── src/
│   ├── lib.rs                    # identidad y fronteras core/platform
│   ├── protocol.rs               # core: contrato CLI ↔ servicio
│   ├── installer.rs              # core: estado persistente/checkpoints
│   ├── main.rs                   # cli: check/status/wait
│   ├── setup.rs                  # installer: entrada UAC/resume
│   ├── setup_gui.rs              # setup-ui: ventana sin privilegios
│   ├── windows.rs                # service/runtime: pipe, WSL, salud, gnx
│   └── windows/
│       ├── installer.rs          # installer/windows: tareas, staging, repair
│       └── prepare-wsl.ps1       # platform: MSI WSL firmado y verificado
├── scripts/
│   └── build-windows.ps1         # build/package/smoke test
├── docs/
│   ├── arquitectura-incremental.md
│   ├── asistente-instalacion.md
│   └── reparacion-post-reinicio.md
└── dist/quetzalcoatl-gnx/        # EXE generados, no versionados
```

## Límites actuales

- **core:** `protocol.rs`, `installer.rs` y las fachadas `core` de `lib.rs`; no dependen de Win32.
- **cli:** `main.rs`; sólo emite consultas autenticadas y no eleva permisos.
- **service/runtime:** `windows.rs`; servicio Windows bajo cuenta dedicada, WSL y Podman.
- **installer:** `setup.rs`, `windows/installer.rs`; estado, UAC, reinicio y recuperación.
- **setup-ui:** `setup_gui.rs`; progreso y diagnóstico, sin permisos administrativos.
- **platform:** `windows/installer.rs` y `prepare-wsl.ps1`; ACL, tareas, MSI WSL y procesos Windows.

Esto conserva los dos EXE y las rutas/identificadores instalados actuales. Extraer crates Cargo independientes es una migración posterior: hoy no aporta funcionalidad y podría romper el instalador validado.

## Comando de usuario

Al completar una instalación nueva, se copian ambos nombres en el directorio ACL-protegido:

```text
C:\ProgramData\QuetzalcoatlGNX\quetzalcoatl-gnx.exe
C:\ProgramData\QuetzalcoatlGNX\gnx.exe
```

El instalador registra esa carpeta en el `PATH` de máquina y notifica el cambio al shell. Una terminal ya abierta debe cerrarse y abrirse de nuevo. La ACL permite ejecutarlo únicamente a SYSTEM, administradores y el SID autorizado; añadir el directorio a PATH no convierte la CLI en un canal sin autorización.

Uso:

```powershell
$ gnx status
$ gnx check
$ gnx wait
```

En una instalación ya existente se requiere una actualización/reparación para crear `gnx.exe` y registrar PATH. Mientras tanto funciona la ruta completa.

## Siguiente migración

1. Extraer `core` como crate sin Windows.
2. Extraer `windows-platform` como crate interno.
3. Mantener `cli`, `service`, `installer` y `setup-ui` como bins separados.
4. Añadir empaquetado firmado, actualización, desinstalador y pruebas en VM.
