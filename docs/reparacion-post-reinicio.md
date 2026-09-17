# Reparación tras el primer reinicio

## Fallos encontrados y correcciones

1. **Motor WSL ausente:** las características opcionales estaban habilitadas, pero `wsl --install --no-distribution` falló con `Wsl/CallMsi/Install/REGDB_E_CLASSNOTREG`. Se instaló el MSI oficial de Microsoft WSL 2.7.14.0 x64, comprobando SHA256 y firma Authenticode válida de Microsoft Corporation. Resultado MSI: 0, sin otro reinicio.
   - `src/windows/prepare-wsl.ps1` incorpora ahora este camino de instalación a nivel de equipo, sin Store ni distribución bajo SYSTEM. Usa versión/hash fijados, `/qn /norestart`, límites de tiempo y `wsl-msi.log`.
   - Si WSL ya existe, comprueba su versión y servicio; no lo reinstala ni lo degrada a una versión anterior.
   - Se probó el MSI manualmente y el nuevo script sobre la instalación existente. La rama automática de descarga/instalación completa todavía debe repetirse en un host limpio.
2. **Servicio rechazado por la comprobación de identidad:** el token de la cuenta estándar de servicio devolvía `TokenIsElevated=true`, aunque no pertenecía a Administradores. La comprobación ahora usa `CheckTokenMembership` con el SID del grupo Administradores, además de exigir el SID dedicado esperado. No se concedieron permisos administrativos a la cuenta.
3. **Podman rootless arrancaba desde `/root`:** `runuser` cambia identidad, pero no directorio de trabajo. La sonda ahora cambia a `/home/quetzalcoatl-gnx` y fija `HOME` antes de ejecutarse como el usuario Linux dedicado.
4. **Diagnóstico incompleto:** los errores de arranque del servicio ahora se escriben en `private/runtime.log` y se publican como `degraded`. Los procesos WSL usan `WSL_UTF8=1` para evitar mezclar UTF-16 con el log de texto.
5. **Sonda PowerShell con código nulo:** durante las pruebas se detectó que `Start-Process` seguido de `WaitForExit()` podía dejar `ExitCode` nulo. El script del host usa ahora `System.Diagnostics.Process` y conserva el handle hasta leer el resultado.

## Resultado comprobado en este host

- Windows WSL 2.7.14.0 operativo y virtualización disponible.
- Ubuntu 24.04 descargado/registrado **bajo la cuenta dedicada**, como WSL2, en `C:\ProgramData\QuetzalcoatlGNX\private\distro`.
- Bootstrap completado: systemd, cgroups v2 y Podman rootless.
- Servicio `QuetzalcoatlGNX` en ejecución, cuenta dedicada fuera de Administradores.
- `quetzalcoatl-gnx.exe check`: código **0**, `state: ready`, `verified: true`.
- Consulta repetida mediante una tarea temporal **con token interactivo limitado**: no administrador, código 0 y estado `ready`.
- Ese mismo token no pudo leer `private/runtime-status.json`: acceso denegado, como se esperaba. La consulta funciona a través del pipe, no del archivo privado.
- Estado persistido del instalador: **`complete`**.
- Tareas temporales de reanudación/interfaz eliminadas automáticamente al terminar; tarea de prueba también eliminada.
- **23 pruebas unitarias aprobadas**, compilación release y arranque del EXE comprobados mediante `scripts/build-windows.ps1`.

No se borraron cuentas ni distribuciones ni se reinició Windows durante esta reparación. Se conservó el entorno creado y se actualizaron los ejecutables. Los registros anteriores se conservaron; pueden contener los errores históricos ya resueltos.

## Alcance

Esto verifica la recuperación y el funcionamiento en este host, no certifica todas las instalaciones desde cero ni el aislamiento frente a un administrador. Siguen pendientes la matriz de fallos/reinicios en VM, una auditoría de seguridad entre identidades y el despliegue de una aplicación Quadlet. `ready` confirma la infraestructura, no una aplicación de negocio.
