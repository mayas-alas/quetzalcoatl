# Asistente gráfico y reanudación de instalación

## Uso

Abre `dist/quetzalcoatl-gnx/quetzalcoatl-gnx-setup.exe` con doble clic, desde el usuario que consultará el entorno. Mantén `quetzalcoatl-gnx.exe` junto al instalador. La ventana no necesita elevación; **Instalar** solicita UAC para un motor separado. Si UAC usa otra cuenta administradora, se conserva el SID del usuario original.

La interfaz muestra fases reales, actividad indeterminada, errores, reintento y un visor de las últimas 64 KiB de `setup.log`. No muestra porcentajes inventados. Cerrar la ventana no cancela al motor.

Cuando Windows requiere reinicio:

1. El motor guarda el estado y prepara las tareas de continuación.
2. La ventana ofrece **Reiniciar y continuar (30 s)**. Guarda tu trabajo antes de aceptarlo.
3. La cuenta atrás es local y cancelable. Cerrar la ventana también la cancela. Al finalizar se solicita UAC; si se cancela, no se reinicia.
4. Se solicita `shutdown /r /t 0`, **sin `/f`**. Las aplicaciones pueden impedir el reinicio para proteger documentos. No usamos un timeout positivo de `shutdown`, que implica cierre forzado en Windows.
5. Puedes posponerlo indefinidamente y reiniciar manualmente.
6. Al arrancar continúa el motor; al iniciar sesión vuelve la interfaz.

La CLI online también utiliza el motor persistente:

```powershell
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$p = Start-Process .\dist\quetzalcoatl-gnx\quetzalcoatl-gnx-setup.exe `
    -ArgumentList "install $sid" -Wait -PassThru
$p.ExitCode
# 3010 = requiere reinicio, 0 = verificación completada, 1 = error.
# Usar -Wait: el EXE ahora tiene subsistema gráfico de Windows.
```

En una consola elevada se puede recuperar el motor manualmente:

```powershell
Start-Process 'C:\ProgramData\QuetzalcoatlGNX-Setup\quetzalcoatl-gnx-setup.exe' `
    -ArgumentList '--resume' -Wait -PassThru
```

El modo offline anterior sigue disponible, pero no tiene esta reanudación. `preflight` sigue sin modificar el host.

## Componentes

- `src/setup_gui.rs`: ventana `egui/eframe`, sin Electron, navegador ni servidor HTTP.
- `src/installer.rs`: contrato de estado, versión y validación de SID.
- `src/windows/installer.rs`: staging seguro, exclusión mutua, persistencia, tareas Windows, reinicio y seguimiento del servicio.
- `src/windows.rs`: preparación de Windows y ejecución del runtime bajo la cuenta dedicada.

Estados: `preparing → reboot_required → provisioning → downloading → configuring → verifying → complete`, con `failed` para errores. Algunas fases se omiten si ya están completadas. La identidad del arranque permite distinguir un reinicio real de otro intento en la misma sesión.

### Persistencia y permisos

Se usa un directorio de staging separado para no confundir la instalación del asistente con el runtime:

```text
C:\ProgramData\QuetzalcoatlGNX-Setup\
  quetzalcoatl-gnx-setup.exe
  quetzalcoatl-gnx.exe
  state.json
  setup.log
  worker.lock / engine.lock
  boot-id.txt
```

El directorio se crea con DACL protegida desde el primer momento: SYSTEM y administradores tienen control total; el usuario autorizado, lectura/ejecución. Se rechazan puntos de reanálisis en las rutas de staging y directorios con propietario o permisos de escritura no confiables. El estado no admite rutas de ejecutables ni contraseñas. La escritura usa `sync_all` y reemplazo atómico Windows; los locks se liberan por el sistema si muere el proceso.

**Ajuste respecto al diseño inicial:** en esta versión la GUI lee el estado protegido en lugar de abrir un segundo named pipe privilegiado. No interpreta ese archivo como comandos. Las acciones administrativas siempre pasan por UAC y el ejecutable; las consultas normales del producto conservan su pipe autenticado. El servicio escribe su informe en `private/runtime-status.json`; el motor lo valida y refleja la fase en el estado público de solo lectura. Solo un `ready` verificado y reciente completa la instalación.

### Tareas temporales

| Nombre | Disparador | Identidad | Acción |
| --- | --- | --- | --- |
| `QuetzalcoatlGNX-Resume` | Arranque | SYSTEM, privilegios altos | EXE protegido `--resume` |
| `QuetzalcoatlGNX-SetupUI` | Inicio de sesión del usuario autorizado | Token interactivo limitado | EXE protegido `--gui` |

Se registran antes de preparar Windows y se eliminan al completar. Ante un fallo permanecen para diagnóstico/reanudación. No almacenan contraseña del usuario ni de la cuenta dedicada. Una colisión inicial con nombres de tareas existentes se rechaza.

SYSTEM prepara Windows y crea el servicio. **Ubuntu no se registra bajo SYSTEM:** su descarga, registro y configuración se ejecutan bajo `svc_quetzalcoatl_gnx`, como antes. El motor comprueba SID, ruta del servicio y cuenta de inicio antes de reutilizarlo.

## Recuperación: alcance real

- Reinicio por características Windows: reanuda sin repetir la creación de recursos.
- Motor interrumpido antes de crear recursos: puede volver a empezar.
- Servicio ya creado: se valida su configuración y se sigue esperando su salud.
- Error de bootstrap con distro marcada y utilizable: el reintento puede reiniciar el servicio, comprobar Ubuntu y repetir el bootstrap idempotente, sin borrar/importar la distro.
- Descarga incompleta no utilizable, cuenta creada sin servicio o carpetas a medio preparar: **se detiene para inspección administrativa**. No se rotan credenciales ni se adopta una cuenta a ciegas.
- No hay rollback/desinstalador, reparación automática universal ni actualización automática del EXE de staging. No reemplazar archivos mientras el motor esté ejecutándose.
- Aún debe validarse el registro de WSL desde el contexto de servicio y la seguridad entre usuarios en una máquina de prueba. No se instala una aplicación Quadlet.

## Compilación reproducible

Windows x64 con Rust y toolchain C/C++ adecuada:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1
```

El script ejecuta pruebas, compila release, comprueba que el EXE real arranca y copia ambos EXE a `dist/quetzalcoatl-gnx/`. No instala ni reinicia Windows.

MSVC usa su toolchain habitual. Para GNU se necesita `rustup component add llvm-tools-preview`: el script usa LLD y LLVM dlltool y genera la librería de importación de `shlwapi.dll` a partir del Windows local dentro de `target/build-support`. No añade dependencias DLL propias al paquete.

En este host, el enlace con las herramientas GNU mínimas produjo un EXE que compilaba pero fallaba al arrancar. Se corrigió usando LLD; por eso la prueba del EXE real forma parte del script, además de las pruebas unitarias. No sustituir este procedimiento por una compilación GNU mínima sin probar el binario.

## Evidencia en este host

- **20 pruebas unitarias aprobadas** de estado, protocolo, atomicidad de archivos y exclusión de workers.
- EXE release: ayuda y apertura/cierre de ventana comprobados; proceso GUI terminó con código 0.
- Instalación online real: staging creado con ACL restringida, tareas registradas, estado `reboot_required`, salida 3010.
- Reanudación manual en el mismo arranque: salida 3010, sin crear el runtime.
- Tarea `QuetzalcoatlGNX-Resume` ejecutada realmente bajo SYSTEM en este mismo arranque: `LastTaskResult = 3010`.
- Tarea de interfaz ejecutada con principal interactivo limitado; la ventana se cerró normalmente y `LastTaskResult = 0`.
- No se reinició Windows durante estas pruebas. **Pendiente:** reinicio real, descarga bajo el servicio, bootstrap, llegada a `ready`, limpieza de tareas y validación efectiva desde otro usuario estándar.

Esto prueba el asistente y el checkpoint de reinicio, **no certifica todavía la instalación completa**.
