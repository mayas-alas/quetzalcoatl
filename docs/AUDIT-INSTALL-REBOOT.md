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
Get-Content 'C:\ProgramData\GNX-0.3.1\journal.json' | ConvertFrom-Json | Select-Object operation,phase,outcome,code
```

Criterios:

- Servicio presente, detenido o iniciado según la fase, y `PathName` exacto.
- `StartName` igual a `.\gnx-runtime`.
- Usuario habilitado como cuenta técnica, descripción `GNX runtime identity`.
- Distro exacta `GNX-0.3.1`; no se deben adoptar otras distros.
- No debe aparecer contraseña, token, hash privado ni URL en evidencia.

## Gates accionables post-reinicio

Ejecutar después del reinicio, en PowerShell elevado y en el contexto del
propietario de la distro WSL. Un error de consulta es `BLOCKED`, no ausencia;
esta plantilla sólo imprime `gate`, `result` y razones allowlisted. No declara
aceptación sin ejecutarse en un host Windows real.

```powershell
$ErrorActionPreference = 'Stop'
$program = 'C:\Program Files\GNX-0.3.1'; $data = 'C:\ProgramData\GNX-0.3.1'
$setup = 'C:\ProgramData\GNX-Setup-0.3.1'; $servicePath = Join-Path $program 'gnx-service.exe'
$results = [Collections.Generic.List[object]]::new()
function Gate([string]$Name, [scriptblock]$Check) { try { & $Check; $results.Add([pscustomobject]@{gate=$Name;result='PASS'}) } catch { $reason=$_.Exception.Message; $status=if ($reason -match '(_query_failed|_unavailable|acl_unreadable|root_missing)') { 'BLOCKED' } else { 'FAIL' }; $results.Add([pscustomobject]@{gate=$Name;result=$status;reason=$reason}) } }
function Require([bool]$Condition, [string]$Reason) { if (-not $Condition) { throw $Reason } }
Gate 'service' { $svc = Get-CimInstance Win32_Service -Filter "Name='GNXRuntime'"; Require ($null -ne $svc) 'service_missing'; $path = [IO.Path]::GetFullPath(($svc.PathName -replace '^"|"$','').Split(' ')[0]); Require ($path -ieq $servicePath) 'service_path_mismatch'; Require ($svc.StartName -ieq '.\gnx-runtime') 'service_account_mismatch'; Require ($svc.State -in @('Running','Stopped')) 'service_state_unexpected' }
Gate 'account-sid' { $user = Get-LocalUser -Name 'gnx-runtime'; Require $user.Enabled 'account_disabled'; Require ($user.Description -eq 'GNX runtime identity') 'account_description_mismatch'; Require ($user.SID.Value -match '^S-1-') 'account_sid_unavailable'; Require (Test-Path (Join-Path $data 'operator.sid')) 'operator_sid_missing' }
Gate 'wsl-distro' { $rows = @(wsl.exe --list --verbose); Require ($LASTEXITCODE -eq 0) 'wsl_query_failed'; $matches = @($rows | Where-Object { $_ -match 'GNX-0\.3\.1' }); Require ($matches.Count -eq 1) 'wsl_distro_missing_or_duplicate'; Require (@($rows | Where-Object { $_ -match 'GNX-' -and $_ -notmatch 'GNX-0\.3\.1' }).Count -eq 0) 'unexpected_gnx_distro' }
Gate 'journal-lock' { $journal = Get-Content (Join-Path $setup 'journal.json') -Raw | ConvertFrom-Json; Require ($journal.operation -eq 'PROVISION' -and $journal.phase -eq 'PROVISIONED') 'journal_not_provisioned'; $lock = Join-Path $setup 'setup.lock'; if (Test-Path $lock) { try { $handle = [IO.File]::Open($lock,'Open','ReadWrite','None'); $handle.Dispose() } catch { throw 'setup_lock_held' } } }
Gate 'acl' { foreach ($path in @($program,$data,$setup)) { Require (Test-Path -LiteralPath $path -PathType Container) "root_missing:$path"; $acl = Get-Acl -LiteralPath $path; Require ($acl.Access.Count -gt 0) "acl_unreadable:$path" } }
Gate 'path-autorun' { $machinePath = [Environment]::GetEnvironmentVariable('Path','Machine') -split ';'; Require (@($machinePath | Where-Object { $_.TrimEnd('\') -ieq $program }).Count -eq 1) 'machine_path_missing_or_duplicate'; foreach ($key in @('HKLM:\Software\Microsoft\Windows\CurrentVersion\Run','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run')) { $entry = Get-ItemProperty -LiteralPath $key -ErrorAction SilentlyContinue; Require ($null -eq $entry.GNXRuntime) 'unexpected_gnx_autorun' } }
Gate 'residue' { $unexpected = @(Get-ChildItem -LiteralPath $setup -Force | Where-Object { $_.Name -notin @('journal.json','setup.lock','snapshot.json','state.json') }); Require ($unexpected.Count -eq 0) 'unexpected_setup_residue' }
Gate 'runtime-ready' { & (Join-Path $program 'gnx.exe') doctor --config (Join-Path $data 'gnx.toml') *> $null; Require ($LASTEXITCODE -eq 0) 'doctor_failed'; & (Join-Path $program 'gnx.exe') status --config (Join-Path $data 'gnx.toml') *> $null; Require ($LASTEXITCODE -eq 0) 'status_failed' }
$results | ConvertTo-Json -Compress; if (@($results | Where-Object result -ne 'PASS').Count) { exit 1 }; Write-Output 'POST_REBOOT_READY observed on this host'
```

Todos los gates deben ser `PASS` y `doctor`/`status` deben terminar con código
cero antes de observar `READY`. El comando `health` no forma parte del contrato
CLI actual; la salud se observa mediante `status`. `PROVISIONED` sigue siendo
`ACTION_REQUIRED`/`SETUP_PROVISIONED`; `FAIL` y `BLOCKED` se reportan como tales.
No conservar journal completo, SID/token, contraseñas, hashes privados, URLs ni
líneas de comando con secretos.

## Hallazgos y riesgos

- La provisión Rust conserva journal y artefactos ante error; no afirma `READY` antes del reinicio.
- La antigua ruta alternativa `provision-gnx-runtime.ps1` fue retirada porque podía crear estado antes de consumir el rootfs autenticado; el flujo oficial usa exclusivamente `install.ps1` y `gnx-setup.exe`.
- La aceptación real aún requiere host Windows desechable, interrupción durante cada fase, reinicio, ejecución de `doctor`, reinstalación y desinstalación.
- No se debe borrar manualmente una instalación parcial: usar `gnx-setup --recover` o `--rollback` y registrar el código resultante.

**Conclusión:** el diseño de instalación es conservador y auditable, pero no puede declararse operativo hasta completar la prueba real de host y reboot.
