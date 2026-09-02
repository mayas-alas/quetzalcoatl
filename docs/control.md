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

La tarea de sesión mantiene WSL activo, actualiza sólo la línea `mesh.gnx`
marcada por GNX en `hosts` y comprueba HTTPS. Conserva un `hosts.before` para
recuperación; no reemplaza entradas ajenas. Arranque sin sesión Windows aún no
validado. La tarea reintenta si su sesión WSL termina.

La CA queda restringida a `mesh.gnx`; se instala únicamente su certificado
público en Windows. La clave CA permanece en WSL. HTTPS valida cadena, nombre
y revocación; HTTP sólo publica `/pki/root.crl`. No se usan excepciones TLS.
El certificado se renueva a menos de 30 días de caducar; la CRL dura 30 días y
se actualiza diariamente. La renovación de la propia CA requiere intervención.

## Dependencias fijadas

Los archivos de servicio contienen digests inmutables: servidor 0.77.1,
consola v2.91.1 y entrada TLS 2.11.4. No hay actualización automática de imágenes
ni habilitación de gateway de agentes. Se conserva la atribución del proveedor
en su consola administrativa.

## Límites y recuperación

Para parar: detener la tarea `GNX Control Host` y los tres servicios WSL.
No borrar datos ni CA para reiniciar. Repetir la preparación no recrea la cuenta.
Los fallos indican un gate y conservan material protegido para diagnóstico;
fallos durante la revocación final requieren revisar el estado antes de reintentar.

El reboot de Windows del 2026-09-02 recuperó los servicios, HTTPS y la conexión
con la misma IP. Falta comparar el ID protegido del peer tras ese reboot.
La rutina de backup está implementada; su ejecución quedó pendiente por UAC
cancelado, y faltan copia USB y restauración. `mesh.gnx` sólo resuelve en este
host; no se publicaron puertos del router ni acceso de terceros.
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
pegarla en chat. El respaldo cubre estado/CA/configuración del control plane,
no el cliente Windows ni su almacén DPAPI. El readback criptográfico no es
una restauración operativa; esa prueba sigue pendiente.

## Fuentes

- [Configuración combinada del servidor](https://github.com/netbirdio/netbird/blob/v0.77.1/combined/config.yaml.example)
- [Bootstrap con PAT](https://docs.netbird.io/selfhosted/automated-setup)
- [Entrada HTTPS y gRPC](https://docs.netbird.io/selfhosted/external-reverse-proxy)
- [Ciclo de vida de systemd en WSL](https://learn.microsoft.com/en-us/windows/wsl/systemd)
- [Respaldo consistente del servidor](https://docs.netbird.io/selfhosted/maintenance/backup)
- [Formato age desde Rust](https://docs.rs/age/0.12.1/age/)
