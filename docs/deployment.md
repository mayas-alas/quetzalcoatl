# Quadlets a un LXC nuevo

Disponible desde 0.0.16; `.network` y el caso fijo privado se incorporaron en 0.0.17.
La candidata 0.0.18 añade espera de disponibilidad PVE mediante healthcheck nativo.
Entrada administrativa por la CLI instalada:

```powershell
$gnx = "$env:ProgramFiles\GnX Core\gnx.exe"
& $gnx deploy 'C:\ruta\a\servicios'
```

El recorrido pendiente de `app.gnx` con **Containerfile → imagen OCI → Podman y
Quadlets** en el LXC existente se prioriza en [features](../features.md#prioridad-de-entrega).
La construcción con Podman pertenece al builder autorizado, no a la admisión de
esta CLI. Este comando crea un LXC nuevo: no construye Containerfiles ni actualiza
el guest existente al recibir una carpeta cambiada. Aplicar imágenes/Quadlets a
ese guest y publicar la aplicación en Caddy requieren un recorrido explícito y
autorizado; no se presentan aquí como comandos ya implementados.

## Stack predeterminado

En las fuentes, `gnx deploy` sin argumentos selecciona `private-services` incorporado
en el ejecutable; no busca una carpeta relativa al directorio de PowerShell.
El servicio genera claves propias de Pocket ID, RustFS y NetBird y utiliza la CA
pública del nodo. Conserva la entrada exacta, ligada al SID y registro WSL, en
`%ProgramData%\GnX Core\default-private-services.json`, protegido para SYSTEM y
administradores. No contiene credenciales Windows/PVE ni crea una cuenta humana.
Este archivo contiene secretos: no publicarlo ni copiarlo al checkout.

La selección reutiliza la admisión, recibos y despliegue LXC existentes. No cambia
definiciones ni regenera secretos al repetir, tampoco sustituye un stack registrado
desde otra entrada. Una entrada parcial o inválida se conserva y bloquea, sin
asignar otro VMID a ciegas. La actualización y desinstalación normal la conservan;
la recreación explícita la incluye en el inventario de estado a retirar.

Se requiere `gnx start`, compute configurado mediante `gnx password` y conexión a
los registros de imágenes. No se despliega implícitamente al instalar/arrancar.
La CLI recibe etapas y estados de unidades en vivo por el pipe autenticado; no
imprime configuración, secretos ni salida bruta de procesos. El detalle nativo
permanece en journal y las tareas PVE. Una desconexión no cancela el trabajo nativo;
un timeout conserva un resultado desconocido y no autoriza repetirlo.

Los instaladores publicados antes de este cambio requieren la carpeta explícita.
La disponibilidad HTTPS, el alta/passkey en `https://pocket.gnx/setup`, S3 y la
conectividad entre peers se comprueban aparte.

### Onboarding inicial del stack privado

Las fuentes 0.0.34 separan el despliegue de aplicaciones del onboarding humano.
`deploy` prepara grupos propios para NetBird y RustFS mediante las APIs de Pocket ID;
guarda sus IDs en el estado privado y vincula los clientes sin adoptar grupos por nombre.
Los clientes nuevos nacen restringidos por grupo. No se crean usuarios ni passkeys.

La CLI indica dos pasos humanos, con una espera de hasta 120 segundos por etapa:

1. Completar `https://pocket.gnx/setup` con correo, nombre y una passkey. Solo el
   único administrador humano con passkey puede seleccionarse como operador inicial;
   la cuenta técnica de API no sirve. Si hay varios candidatos, se detiene sin elegir.
2. Iniciar sesión en `https://netbird.gnx` con Pocket ID **compartiendo el correo**.
   NetBird requiere los datos OIDC `email` y `name`; retirar ese permiso puede impedir
   la autenticación. GnX muestra el aviso, no modifica la pantalla de consentimiento upstream.

GnX añade únicamente al operador inicial a sus grupos, conserva los miembros y
grupos permitidos existentes y activa la restricción por grupo de los clientes
propios. Si un cliente existente no estaba restringido, otros usuarios necesitarán
pertenecer a un grupo permitido a partir de esta finalización. La CLI lo advierte.
El primer usuario NetBird debe corresponder exactamente al ID Pocket ID guardado
y al conector registrado, no solo al mismo correo. Solo a ese usuario se le aprueba
y concede el rol **Admin**; se conserva su asignación de grupos y no se altera la
política global de aprobación ni el propietario técnico. Una cuenta bloqueada sin
aprobación pendiente no se desbloquea automáticamente.

Si vence la espera, se informa **servicios desplegados; onboarding pendiente**.
`gnx deploy` con la misma entrada puede continuarlo en el mismo guest en ejecución,
sin reinstalar ni transferir otra vez secretos. No hay daemon, timer ni aprobación
en segundo plano después de terminar la operación. Tras completarlo, las siguientes
invocaciones no reescriben roles o membresías administrados por el operador.

Las mutaciones nuevas registran intención antes de la llamada y comprueban su
resultado mediante lecturas nativas. Si una respuesta se pierde, no se repite a
ciegas: un ID conocido permite reconocer un resultado ya aplicado; una creación
de grupo sin ID devuelto queda bloqueada, sin adoptar por nombre ni duplicar.
Un token de preparación rechazado/caducado también bloquea, sin resetear cuentas
ni regenerar credenciales. El recibo de despliegue y el de onboarding no acreditan
login humano nuevo, S3 ni conectividad entre peers. Esta implementación en fuentes
no acredita instalación o aceptación runtime de la 0.0.34.

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
- Imágenes fijadas con `Image=registro/imagen@sha256:<digest>` y `Pull=missing`
  (valor por defecto) o `Pull=never`. La admisión en fuentes también permite
  seguimiento explícito de un tag como se describe abajo; no cambia los pins
  del stack privado distribuido.
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

### Política de imagen al arrancar

La admisión Rust en fuentes permite optar por `Image=registro/repositorio:tag`
junto con `Pull=newer`; no se atribuye esta ampliación al CLI 0.0.18 versionado.
El registro debe ser un nombre DNS ASCII explícito (con punto, `localhost` o
puerto explícito); el repositorio usa minúsculas y el tag debe estar presente.
No se admiten URLs, credenciales en la referencia, nombres cortos ni un tag sin
esta política. Tampoco `Image`/`Pull` duplicados ni `Pull=newer` con un digest:
volver a consultar el mismo pin no permite adoptar otra versión.

Quadlet pasa esa política a Podman: al crear el contenedor compara el digest del
tag en el registro y descarga si difiere. También puede ocurrir en un reinicio del
servicio, no solo al arrancar el nodo. No actualiza un contenedor que sigue activo,
no añade un timer y no fuerza el inicio de servicios detenidos. Según la
[política nativa](https://docs.podman.io/en/stable/markdown/podman-run.1.html#pull-policy),
un error de descarga permite usar una imagen local existente; sin ella puede
fallar el arranque. Los límites de tiempo de la unidad siguen aplicándose.

Esto no selecciona canales automáticamente, no garantiza que un digest diferente
sea una versión posterior y no ofrece rollback automático ni reversión de
migraciones de datos. Journal y el estado nativo siguen siendo el diagnóstico.
No actualiza por sí solo los LXC ya registrados: reenviar otra carpeta sigue
solicitando otro despliegue, no una migración de la instancia existente.

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
entrada no crea automáticamente un reemplazo. No borrar recibos ni modificar
una descripción PVE para forzar adopción.

### Recuperar una reserva incompleta de un guest ausente

`gnx deploy` comprueba la ausencia con inventarios nativos PVE y exige que no haya
tareas activas ni discos locales asociados al VMID. Un error, timeout, respuesta
inválida o código `2`/`125` no demuestra ausencia y no habilita la recuperación.
Los recibos deben pertenecer al SID/entrada actuales. Una reserva incompleta no
puede tener un stack privado registrado. Para un despliegue completado cuyo guest
ya no existe, la confirmación también archiva su completion y retira únicamente
el registro privado que apunta a ese mismo guest ausente; no adopta otro stack.

En una consola interactiva, la CLI informa del VMID e intento y solicita escribir
`RECUPERAR <vmid>`. Enter, otra respuesta o stdin redirigido no archivan la reserva.
Tras confirmar, otra petición autenticada revalida nodo, entrada, intento y estado
nativo. El servicio conserva los bytes del recibo en
`deploy-<digest>.missing-<intento>.json` bajo las mismas ACL, retira solo la reserva
activa y, si corresponde, la completion y el registro privado obsoleto, y solicita
un LXC nuevo por el recorrido normal. PVE puede reutilizar un
VMID libre: sigue siendo un guest nuevo, no la restauración del eliminado.

Se conservan la configuración privada, credenciales e historial; no se borran ni
adoptan guests o discos y no se recuperan datos retirados manualmente. Los intentos
son monótonos y acotados a 32 para no aplicar una confirmación antigua a una reserva
nueva. Un archivo histórico distinto o inválido bloquea, nunca se sobrescribe. Si
el guest reaparece o existe otro recurso, el servicio cancela la recuperación.
Si hay interrupción entre archivo y retirada, se reconoce únicamente el histórico
idéntico tras otra confirmación y revalidación; no se repite una creación desconocida.

Reenviar una carpeta idéntica ya aplicada identifica el mismo guest sin reinstalar
ni borrar sus datos. Una carpeta cambiada solicita otro LXC; no existe todavía una
función de actualización, redimensionamiento o eliminación de despliegues.
El digest depende de nombres y contenido: cambiar una credencial, comentario o
salto de línea puede solicitar otro LXC. Una entrada general completada solo consulta
el estado PVE; no vuelve a transferir, arrancar ni verificar sus servicios o archivos.
En las fuentes 0.0.34, el stack privado ya desplegado y en ejecución permite además
continuar su onboarding inicial por las APIs nativas. No inicia un guest detenido.

## Resultado y límites

La respuesta identifica el LXC. Desde 0.0.17 se exige `ActiveState=active` de
cada unidad solicitada, no el éxito agregado de `is-active` usado en 0.0.16.
Esto no permite deducir salud HTTP, login u autorización de cada aplicación.

La base Fedora reutilizada no está depurada específicamente para LXC: unidades
de ajuste de reloj o montajes de sistema son incompatibles con sus restricciones;
la preparación deshabilita esas unidades concretas dentro del guest.

En las fuentes 0.0.29, antes de preparar el guest Rust espera el fin del arranque
con `systemctl is-system-running --wait`, limitado a 120 segundos. `degraded`
permite continuar para tratar esas unidades incompatibles, no acredita salud.
El progreso distingue preparación de transferencia; un error en la preparación
fija conserva línea/código y detalle nativo acotado sin credenciales. Las
transferencias y OIDC no publican contenido ni salida privada. Una espera fallida
no repite ni recrea el guest y conserva su recibo.

Las fuentes 0.0.34 distinguen el error nativo de bus de systemd todavía ausente o
rechazando conexión. Solo en ese caso vuelven a consultar su estado, con una pausa
de un segundo y un único presupuesto total de 120 segundos compartido con la espera
nativa `--wait`. No repiten creación, arranque ni preparación del guest. Otros errores
o un resultado desconocido interrumpen la operación, sin reintento automático.
Si se agota el plazo, no se preparan aplicaciones y se conserva la reserva; hay
que consultar PVE/journal sin borrar el guest ni usar `update --force`. Esta espera
y sus mensajes no se atribuyen a instaladores ya publicados.

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
