# Desinstalación

> **Irreversible:** elimina únicamente los recursos de Quetzalcoatl GNX, incluida su distribución Ubuntu/Podman y sus datos privados. No elimina WSL global, sus características Windows, otras distribuciones ni herramientas externas.

Desde PowerShell elevado:

```powershell
Start-Process 'C:\ProgramData\QuetzalcoatlGNX-Setup\quetzalcoatl-gnx-setup.exe' `
  -ArgumentList '--uninstall --confirm' -Verb RunAs -Wait
```

Sin `--confirm` se rechaza la operación.

## Flujo

```text
Administrador confirmado
  → solicita al servicio la eliminación
  → servicio (cuenta dedicada) ejecuta wsl --unregister quetzalcoatl-gnx
  → espera servicio detenido
  → elimina servicio, cuenta dedicada, PATH de GNX y directorio runtime
  → tarea SYSTEM espera que termine el EXE, borra staging y perfil dedicado
```

La distribución se desregistra desde el **servicio bajo la cuenta dedicada**, porque esa identidad es su propietaria. El instalador no usa `wsl --shutdown`, por lo que no detiene distribuciones ajenas.

La petición de eliminación viaja por el pipe del servicio y exige un token de administrador elevado. El pipe permite conexión a Administradores únicamente para esa operación; `check`, `status` y `wait` siguen autorizados sólo para el SID cliente configurado.

## Seguridad y recuperación

- Nombres, rutas y distribución son constantes; no acepta rutas ni identificadores de recursos externos.
- Si el servicio no se detiene tras cinco minutos y medio, se aborta **sin borrar cuenta, archivos ni PATH**.
- Si un recurso esperado no existe, la limpieza continúa donde es seguro hacerlo.
- Si la desregistración WSL falla, el servicio conserva la distribución y el instalador no procede.
- La tarea temporal `QuetzalcoatlGNX-Cleanup`, bajo SYSTEM, espera la salida del EXE, elimina el perfil de Windows de la cuenta ya borrada mediante `Win32_UserProfile` y elimina el staging. Se desregistra a sí misma antes de borrar su script. Si el perfil sigue cargado o el EXE no termina, conserva staging para diagnóstico en lugar de borrar a ciegas. La tarea conserva además el disparador al arranque y reintenta tras el próximo reinicio; no detiene el servicio WSL global ni sesiones de otras distribuciones.

## Verificación posterior

```powershell
Get-Service QuetzalcoatlGNX -ErrorAction SilentlyContinue
wsl --list --quiet
Get-LocalUser svc_quetzalcoatl_gnx -ErrorAction SilentlyContinue
Get-Command gnx -ErrorAction SilentlyContinue
```

Todos deben no devolver recursos GNX. Puede ser necesario abrir una terminal nueva para observar el PATH actualizado.

## Estado de pruebas

Se compiló y probó el código; el uninstall con la limpieza diferida **no se ejecutó** sobre este host funcional para preservar la instalación validada. Debe probarse primero en una VM desechable y después contra instalación real, incluyendo el fallo intencional de `wsl --unregister`.
