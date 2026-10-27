# Quadlets a un LXC nuevo

Disponible desde 0.0.16 para el subconjunto de archivos descrito aquí.
Entrada administrativa por la CLI instalada:

```powershell
$gnx = "$env:ProgramFiles\GnX Core\gnx.exe"
& $gnx deploy 'C:\ruta\a\servicios'
```

## Carpeta de entrada

```text
servicios/
  pocket-id.container
  rustfs.container
  netbird-server.container
  netbird-dashboard.container
  config/
    pocket.env
    rustfs.env
    netbird.yaml
```

Es una estructura admitida, no un catálogo ni un instalador específico de esas
aplicaciones. Los archivos de configuración reales pueden contener secretos y
deben mantenerse fuera de Git, protegidos para administradores/SYSTEM.
No usar una carpeta compartida o modificable por otros usuarios durante la lectura.

- De uno a ocho `.container` en la raíz; hasta 32 archivos y 48 KiB de texto.
- Máximo 16 KiB por archivo; UTF-8, rutas relativas simples y sin enlaces/reparse points.
- Configuración adicional solo bajo `config/`, hasta cuatro niveles.
- Imágenes fijadas con `Image=registro/imagen@sha256:<digest>`.
- `[Install] WantedBy=multi-user.target` para el autostart nativo.
- Mounts y `EnvironmentFile` limitados a `/var/lib/gnx-app/` y
  `/etc/gnx-app/config/` dentro del guest.
- No se admiten `PodmanArgs`, dispositivos, capacidades adicionales, scripts de
  host ni directivas `ExecStart` de systemd. `Exec` de contenedor no ejecuta código
  en Windows ni en el nodo WSL.
- Esta primera admisión no incluye `.network`, `.volume`, builds de imágenes,
  binarios, archivos OCI ni un selector/subida web.

## Aplicación

La CLI lee los archivos bajo la identidad de su operador y envía bytes, no rutas
que SYSTEM deba resolver. El servicio vuelve a validar la entrada en el pipe
administrativo existente. Una sola mutación puede estar activa.

Rust usa operaciones PVE de argumentos fijos: valida el almacenamiento del runtime,
verifica/copia la base local, reserva un VMID libre y ejecuta `pct create` para un
LXC no privilegiado. Recursos acotados en `config/lxc.json`: dos CPU, 4 GiB de RAM,
512 MiB de swap y disco disperso de 12 GiB. No se modifica un guest preexistente.

La base WSL se reutiliza como filesystem de Fedora para el guest, sin clonar una
instancia enrolada. DHCP y Podman/systemd se ejecutan en el LXC. Las imágenes de
aplicaciones se obtienen desde sus registros según los digests aportados: este
despliegue requiere conectividad; no se declara un despliegue arbitrario offline.

Los archivos llegan por stdin a `pct exec` dentro del guest. Una preparación fija
del producto crea sus directorios; no se ejecutan scripts aportados por el usuario
en el nodo. Los servicios se arrancan mediante systemd y quedan bajo sus Quadlets.
Los directorios de bind mounts bajo `/var/lib/gnx-app/` se crean antes del arranque;
los mounts de configuración deben corresponder a archivos de la entrada.

## Propiedad y repetición

Un recibo protegido por SID, digest de entrada y VMID se guarda antes de crear el
guest. La descripción PVE y el flag no privilegiado se contrastan antes de aplicar.
Un fallo o timeout conserva el recibo y el guest parcial: no lo elimina ni reserva
otro VMID silenciosamente. Consultar tareas PVE/journal antes de repetir.

Reenviar una carpeta idéntica ya aplicada identifica el mismo guest sin reinstalar
ni borrar sus datos. Una carpeta cambiada solicita otro LXC; no existe todavía una
función de actualización, redimensionamiento o eliminación de despliegues.

## Resultado y límites

La respuesta identifica el LXC y las unidades activas. No acredita salud HTTP,
login u OIDC de cada aplicación. Esta operación no publica nuevos dominios, rutas
o puertos en el gateway del nodo, no crea peers Tailscale y no clona su identidad.
Configuración de acceso de aplicaciones e integración de Pocket ID con NetBird y
RustFS deben comprobarse aparte; no se deducen de `systemctl is-active`.
