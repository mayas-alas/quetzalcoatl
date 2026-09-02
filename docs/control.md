# Control plane local

`https://mesh.gnx` sirve el control plane y la consola desde WSL. No usa un
servicio cloud ni un login server en Windows. Corte privado de un solo host.

## Preparar

Con el bundle Windows construido, WSL con systemd y Podman 4.9.3 o compatible:

```powershell
cargo build --release --locked --manifest-path ops/control/Cargo.toml
# PowerShell elevado:
.\ops\control\prepare-host.ps1 -OwnerEmail 'operator@email.gnx'
```

La preparación conserva configuración e identidades existentes, registra el
host, crea el propietario, usa una clave one-off y comprueba la misma identidad
tras reiniciar el cliente. Al terminar elimina la clave y el PAT de bootstrap.
No sobreescribe una instancia cuyo propietario ya exista sin evidencia local.

## Rutinas y estado

| Responsabilidad | Ubicación |
|---|---|
| Configuración y datos del servidor | WSL `/var/lib/gnx/control`, acceso root |
| Material local de operación | `%ProgramData%/GNX/control`, administradores y SYSTEM |
| Credencial del propietario | `%LOCALAPPDATA%/GNX/control/owner.credential.xml`, DPAPI del usuario |
| Estado sin secretos | `%ProgramData%/GNX/control-status.json` |
| Arranque y resolución tras cambio de IP | tarea Windows `GNX Control Host`, al iniciar sesión |
| Reinicio de procesos | servicios `gnx-control`, `gnx-console`, `gnx-entry` en WSL |
| Certificado y CRL | `gnx-identity.timer`, diariamente y tras arranque |
| Respaldo cifrado y evidencia | `%LOCALAPPDATA%/GNX/backups` |
| Clave de recuperación separada | `%LOCALAPPDATA%/GNX/recovery/control.agekey`, ACL del usuario/admin/SYSTEM |

Para consultar usuario y contraseña, en tu PowerShell local y con el mismo
usuario Windows: `.\dist\windows\gnx.exe credentials control`.
Enter revela en pantalla temporal; otro Enter limpia y vuelve al shell.
No usar durante grabación/transcripción ni compartir esa pantalla. Rechaza
redirecciones, no copia al portapapeles ni cambia la contraseña. DPAPI sigue
ligado al usuario y equipo originales; no se leen claves de recuperación.

La tarea de sesión mantiene WSL activo y actualiza la línea marcada por GNX en
`hosts`: `mesh.gnx` y, si está instalado el Quadlet de cómputo,
`proxmox.mesh.gnx`. Comprueba HTTPS del control y conserva un `hosts.before` para
recuperación; no reemplaza entradas ajenas. Arranque sin sesión Windows aún no
validado. La tarea reintenta si su sesión WSL termina.

La CA queda restringida al espacio DNS `mesh.gnx`; el certificado de entrada
cubre `mesh.gnx` y `proxmox.mesh.gnx`. Se instala únicamente el certificado
público de la CA en Windows. La clave CA permanece en WSL. HTTPS valida cadena, nombre
y revocación; HTTP sólo publica `/pki/root.crl`. No se usan excepciones TLS.
El certificado se renueva a menos de 30 días de caducar; la CRL dura 30 días y
se actualiza diariamente. La renovación de la propia CA requiere intervención.

## Dependencias fijadas

Los archivos de servicio contienen digests inmutables: servidor 0.77.1,
consola v2.91.1 y entrada TLS 2.11.4. No hay actualización automática de imágenes
ni habilitación de gateway de agentes. Se conserva la atribución del proveedor
en su consola administrativa.

## Límites y recuperación

Para parar el control: detener la tarea `GNX Control Host` y sus tres servicios
WSL. Si está instalado, `gnx-compute` se detiene por separado.
No borrar datos ni CA para reiniciar. Repetir la preparación no recrea la cuenta.
Los fallos indican un gate y conservan material protegido para diagnóstico;
fallos durante la revocación final requieren revisar el estado antes de reintentar.

El reboot de Windows del 2026-09-02 recuperó servicios, HTTPS y conexión;
se verificó un solo peer con el mismo ID original protegido. El respaldo
cifrado y su copia USB se verificaron por SHA-256 y descifrado completo.
Faltan custodia externa de la clave y restauración operativa. El acceso Android
usa ahora la [capa VPN y DNS privado](access.md); no se publicaron puertos del router.
El antiguo `gnx-host.service` quedó deshabilitado, conservando su archivo;
`legacy` no se modificó.

## Respaldo puntual

```powershell
# PowerShell elevado; requiere el build de ops/control:
.\ops\control\backup-host.ps1
# Sólo después de identificar la letra de una USB conectada (ejemplo E):
.\ops\control\export-backup.ps1 -DriveLetter E
```

El helper Rust `gnx-snapshot` cifra en formato age estándar. Pausa el servidor
para copiar bases consistentes y CA en memoria temporal de WSL; lo reanuda
antes de cifrar el flujo en Windows, sin archivo plano en disco. Comprueba el
descifrado completo mediante SHA-256, la identidad del peer y HTTPS.
La copia USB verifica SHA-256, no formatea ni reemplaza archivos distintos.

La USB contiene únicamente respaldo cifrado y evidencia, nunca la clave.
Guardar también la clave fuera del host, en **otra** ubicación segura: perder
el host y su única clave vuelve inútil el respaldo USB. No imprimirla ni
pegarla en chat. El respaldo cubre estado/CA/configuración del control plane.
Excluye el cliente Windows, DPAPI y `/var/lib/gnx/compute` (configuración,
credencial y discos del servicio). La copia USB verificada precede al despliegue
de cómputo. El readback criptográfico no es una restauración operativa;
esa prueba sigue pendiente. [Alcance del servicio](compute.md).

## Fuentes

- [Configuración combinada del servidor](https://github.com/netbirdio/netbird/blob/v0.77.1/combined/config.yaml.example)
- [Bootstrap con PAT](https://docs.netbird.io/selfhosted/automated-setup)
- [Entrada HTTPS y gRPC](https://docs.netbird.io/selfhosted/external-reverse-proxy)
- [Ciclo de vida de systemd en WSL](https://learn.microsoft.com/en-us/windows/wsl/systemd)
- [Respaldo consistente del servidor](https://docs.netbird.io/selfhosted/maintenance/backup)
- [Formato age desde Rust](https://docs.rs/age/0.12.1/age/)
