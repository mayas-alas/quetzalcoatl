# Instalación automática mediante WSL

**Actualización:** el instalador online ahora usa el [asistente gráfico y motor reanudable](asistente-instalacion.md). Al ejecutarlo sin argumentos abre la ventana. El siguiente procedimiento sigue siendo válido para la CLI; sus códigos de salida se han actualizado.

Desde una consola elevada, con ambos EXE de `dist/quetzalcoatl-gnx/` juntos:

```powershell
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$p = Start-Process .\dist\quetzalcoatl-gnx\quetzalcoatl-gnx-setup.exe `
    -ArgumentList "install $sid" -Wait -PassThru
$p.ExitCode
```

El SID debe ser el del usuario autorizado para consultar. Si se eleva usando otra cuenta, proporcionar explícitamente el SID del usuario cotidiano.

- Código **3010**: reiniciar cuando sea conveniente. Las tareas registradas reanudarán el motor al arrancar y mostrarán la interfaz al iniciar sesión. No se reinicia automáticamente sin consentimiento ni se crea todavía la cuenta dedicada.
- Código **0** del instalador online: el motor ha esperado la verificación del entorno. La variante offline anterior solo confirma el registro del servicio. Para consultar o esperar salud en cualquier momento:

```powershell
.\dist\quetzalcoatl-gnx\quetzalcoatl-gnx.exe wait
```

`wait` consulta cada tres segundos, confirma `ready`, falla ante `degraded`, errores de comunicación o tras una hora. No inicia instalaciones ni admite comandos remotos. `check` y `status` siguen siendo consultas JSON.

## Flujo

1. Habilitar características Windows de WSL y virtualización, respetando reinicios pendientes.
2. Preparar WSL sin distribución bajo el administrador.
3. Crear cuenta dedicada no administradora, carpetas protegidas y servicio.
4. Desde el servicio, descargar Ubuntu 24.04 mediante `wsl --install Ubuntu-24.04 --web-download --no-launch --name quetzalcoatl-gnx --location ...`.
5. Configurar systemd y Podman rootless mediante el bootstrap existente.
6. Verificar systemd, cgroups v2 y Podman antes de indicar `ready`.

Requiere una versión de WSL compatible con `--name` y `--location`, conectividad para WSL y APT, y virtualización disponible. La ejecución de la descarga desde una identidad de servicio todavía requiere validación integral. No se instala Ubuntu en el usuario cotidiano ni se modifican sus distribuciones.

Estados nuevos: `downloading`, `configuring`, `verifying`. Ninguno equivale a `ready`. El progreso es por fases, no porcentaje de descarga. Los comandos del runtime guardan salida en `C:\ProgramData\QuetzalcoatlGNX\private\runtime.log`, protegido por los permisos de la carpeta privada.

Ante instalación parcial se conserva la evidencia y se exige inspección administrativa; no hay borrado automático ni reintento destructivo. La variante offline `install <rootfs.tar> <sha256> <client-SID>` sigue disponible. No se despliega aún una aplicación Quadlet.

## Prueba en este host

- La primera versión online compiló en release y superó 11 pruebas. La evidencia actual del asistente está en [asistente-instalacion.md](asistente-instalacion.md).
- Ejecutado el nuevo instalador online con privilegios administrativos.
- Windows habilitó las dos características requeridas y solicitó reinicio: salida **3010**.
- No se creó la instalación dedicada en esa ejecución. La prueba posterior del asistente sí creó su staging protegido y las tareas de reanudación, pero no el runtime.
- **Pendiente tras reiniciar:** validar descarga bajo el servicio, bootstrap, autenticación de consultas desde consola no elevada y llegada a `ready`. No se considera validado el flujo completo hasta completar estas pruebas.
