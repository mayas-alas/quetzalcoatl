# Quadlets a un LXC nuevo

Disponible desde 0.0.16; `.network` y el caso fijo privado se incorporaron en 0.0.17.
La candidata 0.0.18 añade espera de disponibilidad PVE mediante healthcheck nativo.
Entrada administrativa por la CLI instalada:

```powershell
$gnx = "$env:ProgramFiles\GnX Core\gnx.exe"
& $gnx deploy 'C:\ruta\a\servicios'
```

## Carpeta de entrada

```text
servicios/
  private.network
  pocket-id.container
  rustfs.container
  netbird-server.container
  netbird-dashboard.container
  config/
    pocket.env
    rustfs.env
    netbird.yaml
```

Es una estructura admitida, no un catálogo. El marcador opcional del
[stack privado](../stacks/private-services/README.md) añade preparación fija de
HTTPS/OIDC; requiere también `config/private-stack.json` y `config/root.crt`.
Los archivos de configuración reales pueden contener secretos y
deben mantenerse fuera de Git, protegidos para administradores/SYSTEM.
No usar una carpeta compartida o modificable por otros usuarios durante la lectura.

- De uno a ocho `.container` en la raíz; hasta 32 archivos y 48 KiB de texto.
- Máximo 16 KiB por archivo; UTF-8, rutas relativas simples y sin enlaces/reparse points.
- Nombres en minúsculas ASCII: letras, números, guion, punto y guion bajo;
  `/` separa directorios. No se admiten segmentos ocultos, vacíos, `.` ni `..`.
- Configuración adicional solo bajo `config/`, hasta cuatro niveles.
- Imágenes fijadas con `Image=registro/imagen@sha256:<digest>`.
- `[Install] WantedBy=multi-user.target` para el autostart nativo.
- Mounts y `EnvironmentFile` limitados a `/var/lib/gnx-app/` y
  `/etc/gnx-app/config/` dentro del guest. Los orígenes bajo `/var/lib/gnx-app/`
  se preparan como directorios, no como archivos de datos individuales.
- No se admiten `PodmanArgs`, dispositivos, capacidades adicionales, scripts de
  host ni directivas `ExecStart` de systemd. `Exec` de contenedor no ejecuta código
  en Windows ni en el nodo WSL.
- `.network` en la raíz admite `[Network]`, `Driver=bridge`, `NetworkName=gnx-app-…`,
  `Internal` e `IPv6` booleanos. `Network=nombre.network` exige ese archivo en el
  bundle; `NetworkAlias` admite un nombre DNS simple.
- No incluye `.volume`, `.pod`, `.build`, builds por Containerfile, binarios,
  archivos OCI ni un selector/subida web.

La admisión es un subconjunto, no un validador completo de Quadlet/systemd.
Rechaza continuaciones de línea y valores con `%`; no interpreta todas las
comillas, escapes, dependencias, puertos o rutas de montaje. El operador debe
aportar una carpeta coherente y archivos de configuración existentes. La revisión
de prefijos no prueba por sí sola que cada referencia pertenezca a un archivo del
bundle. Una entrada admitida puede fallar al generarse o arrancarse en el guest.

## Aplicación

La CLI lee los archivos bajo la identidad de su operador y envía bytes, no rutas
que SYSTEM deba resolver. El servicio vuelve a validar la entrada en el pipe
administrativo existente. Una sola mutación puede estar activa.

Rust usa operaciones PVE de argumentos fijos: valida el almacenamiento del runtime,
verifica/copia la base local, reserva un VMID libre y ejecuta `pct create` para un
LXC no privilegiado. Recursos acotados en `config/lxc.json`: dos CPU, 4 GiB de RAM,
512 MiB de swap y disco disperso de 12 GiB. No se adopta un guest ajeno.

La base WSL se reutiliza como filesystem de Fedora para el guest, sin clonar una
instancia enrolada. El caso general usa DHCP; el privado configura IP fija en PVE
y NetworkManager. Podman/systemd se ejecutan en el LXC. Las imágenes de
aplicaciones se obtienen desde sus registros según los digests aportados: este
despliegue requiere conectividad; no se declara un despliegue arbitrario offline.

Los archivos llegan por stdin a `pct exec` dentro del guest. Una preparación fija
del producto crea sus directorios; no se ejecutan scripts aportados por el usuario
en el nodo. Los servicios se arrancan mediante systemd y quedan bajo sus Quadlets.
Los directorios de bind mounts bajo `/var/lib/gnx-app/` se crean antes del arranque;
los mounts de configuración deben corresponder a archivos de la entrada. Esa
coherencia no se comprueba íntegramente antes de crear el LXC.

## Propiedad y repetición

Un recibo protegido por SID, digest de entrada y VMID se guarda antes de crear el
guest. La descripción PVE y el flag no privilegiado se contrastan antes de aplicar.
Un fallo o timeout conserva el recibo y el guest parcial: no lo elimina ni reserva
otro VMID silenciosamente. Consultar tareas PVE/journal antes de repetir.
Una entrada incompleta vuelve a transferir archivos y preparar directorios; no
hay rollback del contenido del guest ni una reanudación transaccional garantizada.
Si se guardó la reserva pero `pct create` no llegó a crear el guest, la misma
entrada no crea automáticamente un reemplazo. No hay un comando de reparación
o limpieza de recibos. No borrarlos ni modificar una descripción PVE para forzar adopción.

Reenviar una carpeta idéntica ya aplicada identifica el mismo guest sin reinstalar
ni borrar sus datos. Una carpeta cambiada solicita otro LXC; no existe todavía una
función de actualización, redimensionamiento o eliminación de despliegues.
El digest depende de nombres y contenido: cambiar una credencial, comentario o
salto de línea puede solicitar otro LXC. Una entrada completada solo consulta el
estado PVE; no vuelve a transferir, arrancar ni verificar sus servicios o archivos.

## Resultado y límites

La respuesta identifica el LXC. Desde 0.0.17 se exige `ActiveState=active` de
cada unidad solicitada, no el éxito agregado de `is-active` usado en 0.0.16.
Esto no permite deducir salud HTTP, login u autorización de cada aplicación.

La base Fedora reutilizada no está depurada específicamente para LXC: unidades
de ajuste de reloj o montajes de sistema son incompatibles con sus restricciones;
la preparación deshabilita esas unidades concretas dentro del guest.
Separar fallos del sistema guest de los de las aplicaciones; no elevar privilegios
ni ocultar estados fallidos para declarar aceptación.

El caso general no publica dominios ni rutas; no crea peers Tailscale ni clona
su identidad. El stack privado sí registra una única instancia con VMID/digest/MAC,
su ruta `/32`, DNS y los cuatro orígenes HTTPS fijos. Comprueba que `root.crt`
coincida con la CA pública del nodo antes de desplegar. Otro stack registrado
bloquea su sustitución, incluso si cambia la carpeta de entrada.

La configuración OIDC nativa conserva secretos bajo `/etc/gnx-app/config/`.
No se adopta un propietario NetBird preexistente sin estado de preparación, ni
se crea una passkey humana. S3 CRUD, login y conectividad NetBird entre peers
son comprobaciones independientes. STUN UDP no atraviesa el gateway HTTPS ni
el portproxy TCP Windows; publicar su puerto en el guest no prueba acceso externo.
