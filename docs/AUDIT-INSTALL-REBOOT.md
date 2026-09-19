# Auditoría breve — instalación Windows GNX 0.3.1

Fecha: 2026-09-19  
Alcance: instalación inicial, reinicio, usuario dedicado y servicio.

## Resultado actual

**Código:** aprobado en pruebas unitarias; **aceptación de host: pendiente**.

`cargo test --all`: 28 pruebas ejecutadas correctamente (1 ignorada por requerir Linux root).
No se realizó todavía una instalación ni un reinicio real en Windows desde este entorno.

## Flujo esperado

1. `install.ps1` exige Administrador.
2. Verifica hash del manifiesto, artefactos y rootfs.
3. Rechaza raíces legacy o instalaciones parciales existentes.
4. `gnx-setup --provision` crea journal/lock, copia artefactos autenticados y publica:
   - `C:\Program Files\GNX-0.3.1`
   - `C:\ProgramData\GNX-0.3.1`
   - `C:\ProgramData\GNX-Setup-0.3.1`
5. Crea el usuario local `gnx-runtime` con logon de servicio y denegación de logon interactivo/remoto/red.
6. Registra `GNXRuntime` apuntando exactamente a `gnx-service.exe`.
7. El resultado de provisión es `ACTION_REQUIRED`/`SETUP_PROVISIONED`, no `READY`.
8. Tras reinicio, se debe ejecutar bootstrap, `doctor` y health-check antes de declarar `READY`.

## Comprobaciones post-reinicio

Ejecutar en PowerShell elevado y conservar únicamente salida sanitizada:

```powershell
Get-Service GNXRuntime
Get-LocalUser gnx-runtime | Select Name,Enabled,Description,SID
Get-CimInstance Win32_Service -Filter "Name='GNXRuntime'" |
  Select Name,State,StartName,PathName
wsl.exe --list --verbose
Test-Path 'C:\Program Files\GNX-0.3.1\gnx-service.exe'
Test-Path 'C:\ProgramData\GNX-0.3.1\operator.sid'
Get-Content 'C:\ProgramData\GNX-0.3.1\journal.json'
```

Criterios:

- Servicio presente, detenido o iniciado según la fase, y `PathName` exacto.
- `StartName` igual a `.\gnx-runtime`.
- Usuario habilitado como cuenta técnica, descripción `GNX runtime identity`.
- Distro exacta `GNX-0.3.1`; no se deben adoptar otras distros.
- No debe aparecer contraseña, token, hash privado ni URL en evidencia.

## Hallazgos y riesgos

- La provisión Rust conserva journal y artefactos ante error; no afirma `READY` antes del reinicio.
- La ruta alternativa `provision-gnx-runtime.ps1` puede dejar cuenta/directorio temporal si WSL solicita reinicio después de crear el usuario; debe probarse separadamente o retirarse del flujo oficial.
- La aceptación real aún requiere host Windows desechable, interrupción durante cada fase, reinicio, ejecución de `doctor`, reinstalación y desinstalación.
- No se debe borrar manualmente una instalación parcial: usar `gnx-setup --recover` o `--rollback` y registrar el código resultante.

**Conclusión:** el diseño de instalación es conservador y auditable, pero no puede declararse operativo hasta completar la prueba real de host y reboot.
