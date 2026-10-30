# Arquitectura de GnX Core

Descripción de los mecanismos implementados. El alcance y las capacidades
pendientes se definen en [scope.md](scope.md).

## Componentes e identidades

```text
Windows: CLI elevada ── pipe local ── servicio GnXCore (LocalSystem)
                                           │ token y perfil dedicados
                                           ▼
                              cuenta Windows gnxcoresvc
                                           │
                                     WSL gnx-core
                                           │
                                  systemd / Quadlets
                           ┌─────────┬───────┬─────────┐
                        Tailscale  Pi-hole  Caddy  Proxmox VE
                                            │       ▲
                          app.gnx ◄──────────┤       │ TLS
                      compute.gnx ◄──────────┴───────┘
```

| Recurso | Identificador |
| --- | --- |
| Ejecutable | `%ProgramFiles%\GnX Core\gnx.exe` |
| Servicio SCM | `GnXCore` |
| Pipe | `\\.\pipe\GnX.Core.v1` |
| Cuenta dedicada | `gnxcoresvc` |
| Distribución WSL2 | `gnx-core` |
| Estado Windows | `%ProgramData%\GnX Core` |
| Estado Linux | `/var/lib/gnx-core` |
| Target | `gnx-core.target` |

La cuenta dedicada no es administradora ni una cuenta humana: tiene derecho de
logon de servicio, denegación de inicio interactivo local/RDP y está oculta del
selector de usuarios. La ocultación no sustituye permisos. La identidad se
comprueba por SID y registro WSL, no por el nombre de la cuenta/distribución.

El servicio es LocalSystem para las operaciones Windows; sus procesos WSL se
ejecutan con token y perfil de la cuenta dedicada. Cambiar el UID Linux no cambia
al propietario Windows. No se promete aislamiento frente al administrador del host.

## Control y secretos

`apps/gnx` contiene un ejecutable Rust para CLI y servicio. La CLI verifica el
servidor del pipe; el servicio autoriza al cliente. El protocolo limita tamaños,
versiones, operaciones y tiempos. El pipe es local y administrativo, incluso para
`status`. No hay control privilegiado desde HTTP ni shell de usuario.

`enroll` y `password` leen sin eco. Los secretos llegan por el pipe y stdin
heredado a operaciones Linux fijas. La contraseña aleatoria de la cuenta Windows
se protege con DPAPI y ACL. La credencial Proxmox se conserva en un archivo Linux
protegido que consume su contenedor; administradores/root pueden acceder al
entorno del runtime. No se afirma que el secreto nunca exista en variables de entorno.

El estado persistente Tailscale es la identidad del peer. Conservarlo evita
reenrolar en cada inicio; no equivale a implementar una distribución de keys
para futuros LXC.

## Lifecycle y aplicación del paquete

- SCM recupera el servicio Windows; systemd/Quadlets recuperan contenedores.
  Proxmox conserva su gestión nativa de guests. Rust no duplica esos supervisores.
- `gnx-core.target` agrupa los cuatro servicios. Sus dependencias de disponibilidad
  local ordenan gateway/DNS, no la autenticación mesh ni el arranque de compute.
- Cada Quadlet declara su lifecycle y `PartOf=gnx-core.target`, que propaga stop.
- `start` y `stop` delegan en systemd. La intención persistida distingue una parada
  del usuario de una caída del servicio. La recuperación no deshace `stop`.
- `status` no inicia una WSL detenida. Target activo no equivale a HTTPS, mesh
  o compute sanos. Un resultado ocupado o timeout no acredita éxito.
- La preparación verifica el payload y aplica scripts/configuración al instalar
  o cambiar el paquete. `applied-version` evita repetirlo en cada recuperación.
  `start` y `status` no son instaladores ni mecanismos de actualización.

La base procede de Fedora bootc por digest en `config/bootc.json`, con las
adaptaciones de `config/wsl.conf` y `config/base-masks`. Se importa un filesystem
preparado; no se implementa `bootc switch` dentro de WSL. Las imágenes de los
cuatro servicios están fijadas en `config/images.json` y los Quadlets usan
`Pull=never`. Proxmox se configura sin exigir KVM; no se prometen VMs aceleradas.

## Red y HTTPS

En Windows, una línea hosts marcada y forwards propios de `127.0.0.1:443` y
`127.0.0.1:80` dirigen tráfico a WSL. `netsh portproxy` transporta TCP sin terminar
TLS. Se verifican recibos antes de modificar forwards y no se reescribe lo que
ya coincide. No se cambia el DNS global del host.

Caddy sirve `app.gnx` y actúa como proxy de `compute.gnx` hacia Proxmox `:8006`,
verificando nombre y CA del backend. La app es HTML estático con `status.txt`:
la última fase de configuración, no salud en vivo ni una API administrativa.
La escucha del gateway se limita a sus IP NAT y mesh, no a una publicación LAN.

El stack privado registrado añade `pocket.gnx`, `rustfs.gnx`, `s3.gnx` y
`netbird.gnx` al mismo gateway y certificado. Windows gestiona los seis nombres
locales; las rutas Caddy de aplicaciones se importan solo después de registrar
el stack. DNS PVE incluye sus nombres y una reserva DHCP para la MAC registrada.
Una ruta `/32` lleva el tráfico al guest, no sustituye el gateway general del nodo.

El gateway obtiene su IP NAT mediante `ExecStartPre`. El Quadlet mesh espera
interfaz/IP con `tailscale wait`, luego reconcilia DNS y bindings mediante el
script existente. `flock` serializa el acceso; solo se reemplaza configuración o
se solicita reinicio si cambia. La espera pertenece a mesh; no bloquea app/target.

El acceso remoto requiere Split DNS de `gnx` al Pi-hole del nodo, permisos de
DNS TCP/UDP 53, HTTPS TCP 443 y HTTP TCP 80 para descargar la CRL. La confianza en
la CA privada depende de cada cliente. Esas políticas externas no las configura
el instalador y no se deducen de una comprobación local.

## PKI interna

La CA y las claves son persistentes. La instalación incorpora el certificado
público exacto al almacén raíz Windows si falta; no exige una operación manual de
certificados al usuario del host. La app normal no muestra una acción de importación.
El certificado público permanece disponible en `/gnx-labs-root.crt` para
administración: nunca se distribuyen claves privadas ni se acepta una excepción TLS.
Otros dispositivos o almacenes independientes necesitan confianza administrada
por separado, verificando la huella por un canal confiable.

`scripts/tls.run` usa OpenSSL y un bloqueo compartido. El servicio/timer
`gnx-core-crl` renueva la CRL cada seis horas y al arrancar el gateway; su vigencia
es de 24 horas. La hoja servidor se renueva cuando quedan menos de 30 días,
conservando raíz y clave. La hoja revocada no se renueva como si fuera válida.
Los montajes de directorio permiten reemplazo atómico; los servicios nativos
aplican los certificados.

La CRL firmada se publica en `http://app.gnx/pki/root.crl`; es el único recurso
HTTP permitido, los demás responden 404. Su firma aporta autenticidad sin crear
un ciclo TLS para obtenerla. No se añade OCSP ni se desactiva revocación. Los
clientes pueden cachear la CRL y no todos los navegadores la consultan.

## MSI, actualización y retirada

WiX 5 produce el MSI y Burn el EXE que lo contiene. .NET es dependencia de build,
no un runtime del producto. La transacción MSI y la preparación Linux son fases
distintas: instalación finalizada no acredita servicios disponibles.

| Capa | Responsabilidad | Límite |
| --- | --- | --- |
| EXE Burn | Entrada interactiva recomendada, presentación y ejecución del MSI incorporado. Burn gestiona su propio registro de mantenimiento y caché. | No prepara WSL, no aplica configuración Linux y no incorpora un segundo instalador del nodo ni prerrequisitos adicionales. |
| MSI | Única transacción Windows: archivos, registro del servicio y coordinación de su retirada. También permite instalación administrativa directa. | No sustituye al servicio como preparador/supervisor del runtime. |
| Servicio Rust | Verifica propiedad, prepara cuenta/WSL, aplica el payload y delega la operación en systemd. | No duplica Windows Installer ni el lifecycle de contenedores. |

El EXE contiene el mismo MSI distribuido por separado: son alternativas offline,
no componentes acumulativos. No se debe instalar una vía sobre la otra para
completar el producto. Para mantenimiento se utiliza la entrada de Windows de la
vía elegida; cuando instala Burn, el MSI encadenado se oculta de la lista de apps.
Cambiar entre vías y el ciclo completo de mantenimiento Burn no se presentan como
aceptados por haber verificado el empaquetado o el ciclo MSI directo.

La distribución para usuarios publica únicamente `GnxCoreSetup.exe`, `GnxCore.msi`
y `SHA256SUMS.txt`. CLI, digest base, manifiesto y changelog quedan en un artefacto
técnico separado de CI. El binario versionado del repositorio sigue siendo
obligatorio; su presencia no convierte la CLI en un instalador portable.

La retirada explícita (`REMOVE=ALL` sin `UPGRADINGPRODUCTCODE`) llama a una entrada
interna que usa el pipe/servicio existentes. Confirma la parada antes de retirar
hosts, forwards y certificado Windows propios; un fallo bloquea la desinstalación.
La acción de rollback intenta aplicar la intención previa mediante el mismo pipe;
su presencia no prueba recuperación completa. Después MSI retira el servicio.
La retirada requiere `GnXCore` en ejecución y sin otra operación en curso: no
inicia automáticamente el servicio Windows si está detenido. Los requisitos y
recursos conservados están en [Mantenimiento y desinstalación](docs/uninstall.md).

Cuenta, perfil, WSL, DPAPI, CA y datos permanecen para reinstalar. Las actualizaciones
no reemplazan una WSL existente con el rootfs de primera instalación. No existe
una función de purga. El empaquetado local y CI, sus inputs y gates se describen
en [docs/packaging.md](docs/packaging.md).

## Diagnóstico y límites

SCM/Visor de eventos y systemd/journal son las fuentes de diagnóstico. No se añade
otro observador ni se presenta una fase histórica como salud actual.
Desde 0.0.16 existe una entrada CLI de despliegue a LXC; no existe
recepción web ni puente nativo desde la página. El contrato y estado de aceptación
se detallan en [Despliegue](docs/deployment.md). Las integraciones de aplicaciones
no se acreditan por iniciar sus contenedores.

## Despliegue por Rust y PVE

La CLI recoge una carpeta administrativa y transmite un bundle acotado por el
pipe existente. SYSTEM no resuelve paths aportados por el cliente. La admisión se
repite en el servicio; se rechazan enlaces, escapes y directivas de host fuera del
subconjunto admitido. No hay scripts suministrados por el usuario en el nodo.

Rust verifica almacenamiento y base del runtime, reserva VMID/recibo antes de
mutar y llama a `pct create` con argumentos fijos. El guest es no privilegiado,
con CPU/RAM/disco acotados. Un fallo preserva el recibo y el guest parcial, sin
asignar otro identificador silenciosamente. Propiedad se comprueba por SID,
digest de la entrada, VMID, descripción PVE y flag no privilegiado.

Los archivos llegan por stdin a operaciones fijas de `pct exec`. Se preparan
directorios privados, bind mounts de datos y configuración; los Quadlets los
aplica systemd del guest. No se añade un segundo supervisor ni se instalan las
aplicaciones en el núcleo. Se admiten `.container` y `.network` bridge, con alias
DNS y comprobación de referencias de red presentes en el bundle. No se compilan
Containerfiles ni se admiten todas las clases de Quadlet.

`images.rs` valida la pareja `Image`/`Pull` en la admisión de aplicaciones.
Conserva pins por digest con `missing`/`never` y permite un tag explícito plenamente
calificado solo con `newer`. No interpreta tags implícitos ni añade un supervisor:
Quadlet entrega la descarga al `podman run` generado. Esto no cambia las imágenes
del núcleo ni reconcilia guests ya completados. Los límites están en
[la política de imagen](docs/deployment.md#política-de-imagen-al-arrancar).

El filesystem LXC actual procede del mismo rootfs preparado para WSL. No contiene
por ello un despliegue bootc transaccional ni acredita rollback de la base. La base
bootc recreable con recuperación para LXC es un
[requisito pendiente](scope.md#requisitos-pendientes-autorizados), distinto de
cambiar la imagen de un contenedor Podman.

El cierre consulta `ActiveState` por cada unidad solicitada, también después de
la configuración OIDC del stack. No prueba salud funcional. Al reenviar una entrada completada solo se
consulta `pct status`, sin reconciliar archivos ni comprobar otra vez las unidades.
La preparación enmascara ajuste de reloj y montajes de host incompatibles con el
guest no privilegiado; no se describe como una plantilla LXC mínima o libre de fallos.

### Stack privado

El marcador `config/private-stack.json` selecciona una operación fija. El guest
usa `172.30.0.100/24`, configurada en PVE y NetworkManager, y los cuatro contenedores
comparten una red Podman bridge definida por `private.network`, no la red host.
Los bind mounts conservan datos bajo `/var/lib/gnx-app/`. Las imágenes oficiales
están fijadas por digest en [las definiciones](stacks/private-services/README.md).

`stack-access.run` vincula VMID, digest y MAC; verifica descripción y propiedad
PVE antes de reconciliar la ruta y publicar Caddy. El arranque compute reconcilia
ese estado mediante `ExecStartPost`, después de `Notify=healthy` y un healthcheck
nativo de la API PVE, sin otro daemon. No reemplaza una instancia
registrada para aceptar una entrada diferente.

`enroll-stack.run` se ejecuta en el guest y usa las APIs Pocket ID/NetBird. Guarda
estado privado antes de mutar, crea clientes separados y configura RustFS con
`readwrite`, no administración global. No restablece un propietario NetBird ajeno.
El guest recibe solo la CA pública; la preparación verifica nombre/cadena TLS y
la CRL firmada vigente sin rotar la raíz existente. El operador registra su passkey
real en Pocket ID. Preparar clientes OIDC no acredita autenticación humana.

Caddy transporta HTTP/2 gRPC y relay NetBird; no transporta STUN UDP. El puerto
3478 se publica en la IP del guest, no mediante el portproxy TCP Windows. Acceso
externo y conexión entre peers requieren su comprobación independiente.
