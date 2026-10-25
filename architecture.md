# GnX — arquitectura mínima

**Estado: diseño 0.6; implementación acotada autorizada el 2026-10-25.**
Autoridad de alcance y preguntas: [scope.md](scope.md), incluida la instrucción vigente.
Las menciones a «solo documentación» abajo describen la fase anterior. Las
decisiones abiertas y la aceptación no se consideran resueltas por implementar.

Nombres nuevos: servicio `GnXCore`, pipe `\\.\pipe\GnX.Core.v1`, cuenta
`gnxcoresvc`, distro `gnx-core`, estado `%ProgramData%\GnX Core`. Sustituyen los
nombres propuestos en el diseño histórico inferior, sin adoptar ni alterar
`GnXHostAgent` ni su instalación. El desinstalador antiguo fallido queda separado.

**Pruebas diferidas por el usuario (2026-10-25):** no añadir código de tests ni
infraestructura. El paradigma de pruebas se abstrae fuera de este esfuerzo;
solo formato, compilación y comprobaciones externas mínimas/sintéticas, sin
instalaciones de prueba. Las propuestas de tests inferiores no autorizan
implementarlos ahora. Al retomar, leer esta restricción y `scope.md`; distinguir
siempre compilado, instalado, comprobado y aceptado.
Recorrido y criterios derivados: [historias-de-usuario.md](historias-de-usuario.md).
Antes de añadir o cambiar contenido, leer la versión vigente de ambos documentos
y comprobar las versiones/referencias citadas; disciplina en `scope.md`, sección 7.

## 1. Resultado, no plataforma

Un instalador Windows entrega `gnx`, un servicio resiliente y una WSL dedicada.
El MSI y el EXE contienen los inputs necesarios para instalar el núcleo offline;
no son descargadores de un sistema que falte preparar en el equipo de destino.
Desde el host se accede por HTTPS válido a **`app.gnx` y `compute.gnx`**.
La WSL base ejecuta **Tailscale, Proxmox VE (PVE), Caddy y Pi-hole** mediante
systemd/Quadlets. No instala aplicaciones de demostración ni workloads heredados.
**`app.gnx` funciona localmente y muestra avance antes del enrolamiento mesh.**
Falta de Internet, key pendiente o error Tailscale no deben detener Caddy/Pi-hole.

**Objetivo final confirmado:** desde `app.gnx` o `gnx`, entregar un Quadlet o
carpeta de Quadlets/configuración para desplegar servicios en LXC de Proxmox.
`Containerfile` y contexto solo cuando haya que construir; no exigir build a
una imagen OCI ya disponible. Primer caso real: **un LXC con RustFS, Pocket ID
y NetBird**, con Pocket ID integrado únicamente con las otras dos aplicaciones.
La primera entrega del núcleo no equivale a esa función final completada.

```text
ENTREGA                         NO ES ENTREGA
MSI + EXE                       "el build pasó"
identidad + WSL propias          WSL del usuario host
servicios bajo systemd           terminal que debe quedar abierta
ambas URLs verificadas           contenedores simplemente "Up"
recuperación sin perder estado   reinstalar/borrar para recuperarse
```

Todo lo siguiente es diseño propuesto dentro de esos límites. Las decisiones
`Dxx` marcadas abiertas en `scope.md` no son defaults aprobados; D04 ya confirma
Pi-hole por instrucción del usuario.

## 2. Un dueño por responsabilidad

```text
WINDOWS
  Usuario host
    +-- navegador ---------------------------+  tráfico HTTPS
    +-- gnx.exe                              |
          | Named Pipe local, autorizado     |
          v                                  |
      servicio Rust                          |
      seguridad + lifecycle + acceso host    |
          | token/perfil de gnxnodesvc        |
          | wsl.exe + pipes heredados         |
          v                                  |
  Cuenta dedicada gnxnodesvc                 |
    +-- WSL2 gnx-node                        |
          +-- systemd                        |
                +-- Podman / Quadlets        |
                      +-- mesh (Tailscale)   |
                      +-- DNS (Pi-hole)      |
                      +-- gateway (Caddy) <--+
                      |     +-- app.gnx: acceso + envío final de carpeta [D01/D12]
                      |     +-- compute.gnx --TLS--> Proxmox :8006
                      +-- compute (Proxmox, sin guests preinstalados)
```

| Pieza | Hace | No hace |
| --- | --- | --- |
| `gnx` / Rust | Recoge inputs, envía operaciones tipadas, muestra resultado/acción. | Orquestar Linux directamente; recibir secretos por argumentos. |
| Servicio / Rust | Autoriza IPC, gestiona identidad/WSL, delega en systemd y publica acceso local. | Supervisor de contenedores/LXC, shell remoto, motor de plugins, proxy HTTP de control. |
| Cuenta dedicada | Posee perfil, registro WSL y almacenamiento propios. | Usar sesiones/credenciales del host o actuar como cuenta humana. |
| systemd + Quadlets | Dependencias, arranque, límites y recuperación de contenedores. | Reinicios competidores desde scripts y desde Rust. |
| Gateway | TLS, contenido `app.gnx`, proxy de Proxmox. | Onboarding privilegiado, manejo de keys mesh, ejecución de comandos. |
| Proxmox | Interfaz y API de compute con su autenticación nativa. | Provisionar nuestros antiguos workloads. |

Son **cuatro servicios base requeridos**, no candidatos a sustituir por otros.
systemd y Podman son infraestructura de ejecución, no servicios de negocio.
Las aplicaciones enviadas posteriormente viven en LXC de PVE, no como nuevos
contenedores junto a los cuatro del nodo WSL.

Servicio propuesto: `GnXHostAgent`, automático, bajo `LocalSystem` por las
operaciones Windows necesarias. Linux se inicia **bajo la identidad dedicada**,
no bajo SYSTEM. Revisar privilegios efectivos, SID, perfil y entorno; no hacer a
`gnxnodesvc` administrador por comodidad. No prometer aislamiento frente a un
administrador del propio host.

## 3. Control Windows y secretos del nodo

```text
INPUT HUMANO       FRONTERA WINDOWS          FRONTERA LINUX       CONSUMIDOR
gnx, sin eco ---> Named Pipe protegido ---> servicio/token ---> pipe heredado
                      ^                          |                    |
                 valida cliente             no shell libre          v
                                                             archivo protegido
                                                               si es necesario
                                                                    |
                                                                    v
                                                              mesh / Proxmox

retorno <---------- estado sanitizado <--------- código + fase + acción
NUNCA: claves en respuesta, logs, argv, URLs, Git o estado público
```

- Un Named Pipe de control local; nombre propuesto `\\.\pipe\GnX.Platform.Control`.
  DACL restrictiva, rechazo remoto, autorización por token y verificación del
  servidor por la CLI. Acceso de lectura sin elevación: pendiente `D08`.
- Frames versionados y acotados; operaciones y rutas permitidas explícitamente.
  Límites de tiempo, correlación, rechazo de replay y una sola mutación activa.
  Un cliente lento/desconectado no debe bloquear estado ni matar el servicio.
- Propuesta: secretos pequeños por el propio pipe, en memoria; evitar el bundle
  temporal del legacy. La entrega a WSL usa stdin/handles, nunca valores en argv.
  No introducir otro listener de control privilegiado Windows. La entrada de
  carpetas desde la web es un contrato distinto, aún por resolver en `D12`.
- Contraseña de la cuenta Windows: aleatoria, DPAPI + ACL; no la elige ni conoce
  el usuario. DPAPI de máquina **no sustituye** la ACL.
- Key mesh: consumir y descartar por defecto propuesto; preservar la identidad
  ya inscrita. Retención reusable requiere decisión `D06`.
  La CLI la entrega al Tailscale del runtime WSL dedicado, no modifica el rootfs
  distribuido ni reconstruye la imagen OCI. Tailscale documenta auth por archivo
  (`--auth-key=file:...`): el argumento lleva una ruta, nunca el valor. Comprobar
  soporte en la versión elegida, permisos y limpieza. Estado del peer persistente
  y key de enrolamiento son cosas distintas; reiniciar no exige otra key si la
  identidad sigue válida.
- Contraseña Proxmox: input humano protegido; persistencia mínima necesaria para
  reinicios, sin contraseña pública predeterminada. Rotación explícita y sin
  cambiar la identidad mesh. Contrato de entrada de la imagen: `D05`.
- CA privada y credenciales solo donde deban consumirse. Archivos Linux root
  `0600`/directorios `0700`; Windows con DACL explícita. Limpiar staging incluso
  al fallar. No heredar secretos del entorno del usuario host.
- Si la imagen solo admite un secreto por entorno, documentar esa exposición a
  administradores/inspect del runtime y decidir el adaptador; no afirmar
  "nunca existe en env" ni incrustarlo en el Quadlet. Sin secret manager nuevo.

## 4. Datos y acceso: dos URLs, un gateway

```text
Navegador host
  nombres propios -> loopback:443 -> adaptador de acceso -> WSL gateway:443
                                                            |
Cliente tailnet [D03]                                        +--> app.gnx
  Split DNS -> IP mesh del nodo -> ACL mesh -> gateway:443    |
                                                            +--TLS verificado-->
                                                               Proxmox :8006
```

El adaptador Windows propuesto es transporte TCP sin terminar TLS, mantenido
por el servicio; mecanismo exacto: `D09`. Comprobar conflictos de puerto antes
de cambiar nada y reconciliar la dirección WSL después de reiniciar.

Solo loopback y mesh autorizada; no publicar `8006` a LAN/tailnet, ni abrir router,
Funnel, Serve o DNS público. DNS no debe convertirse en resolver abierto. Cambios
Windows de nombres/confianza se limitan a entradas/certificados propios con
identificador de ownership; no reemplazar configuración ajena.

CA privada del nodo; confiar únicamente su certificado público en el host.
Gateway valida nombre y CA de Proxmox: sin `-k` ni TLS insecure. Para clientes
externos, confianza y Split DNS son acciones visibles, no cambios silenciosos
en la cuenta Tailscale. Renovar certificados antes de expirar sin regenerar la
identidad del nodo. HTTPS por sí solo no autoriza desplegar. No añadir IdP/OAuth/
sesiones para el núcleo por inercia; la autorización del envío final se resuelve
en `D12`, antes de habilitar cualquier mutación desde `app.gnx`.

### Arranque local primero; enrolamiento después

```text
paquete instalado -> gnx prepara cuenta/WSL e inputs locales
                                 |
                                 v
                     Pi-hole + Caddy + CA/nombres/acceso host
                                 |
                                 v
                     https://app.gnx DISPONIBLE
                     muestra avance y requisitos pendientes
                                 |
gnx: key sin eco -> pipe -> servicio -> runtime WSL -> Tailscale
                                                        |
                                      conectividad + credencial válidas
                                                        |
                                                        v
                                            mesh enrolada / acceso remoto

app local sigue operativa si mesh está pendiente, falla o pierde conexión
compute avanza según sus requisitos, incluida contraseña Proxmox por CLI
```

Caddy debe servir la página real, TLS y estado, además de su proxy de compute;
no una pantalla provisional Windows ni un gateway que espere a PVE/mesh para
arrancar. Preparar certificados/config válidos de las rutas antes de iniciarlo;
backend compute aún no listo no impide servir app. No iniciar PVE con una
contraseña pública predeterminada para adelantar el arranque.

El bootstrap de nombres del host es local y propio, no depende de una IP mesh.
Pi-hole arranca con escucha/registros locales; añadir exposición/registros mesh
solo cuando exista una IP válida, sin convertirlo en resolver abierto ni devolver
loopback a clientes remotos. No exigir `tailscale0` ni MagicDNS para el arranque
local; no crear un ciclo DNS Pi-hole ↔ Tailscale. No cambiar el DNS global del host.
TLS local debe usar CA interna explícita, no esperar ACME ni certificados `.ts.net`;
el servicio establece confianza del certificado público en Windows antes de abrir app.

Propuesta mínima de avance: el servicio publica atómicamente un `status.json`
sanitizado; Caddy lo sirve de solo lectura, mismo origen y `no-store`. La página
refresca fase, estado, causa/acción y antigüedad sin leer logs crudos ni secretos.
No requiere otro backend para observar. Es seguimiento, no onboarding por web.
Antes de que WSL/Caddy estén disponibles informa la CLI; después, CLI y app
presentan la misma operación. No condicionar la publicación local a un "todo listo".

### Recorrido final: Quadlet/contexto a servicio LXC

```text
app.gnx: enviar archivos       gnx: indicar path local
  Quadlets + config + imágenes; Containerfile/contexto si requiere build
                 |
                 v
  entrega autorizada [D12] -> validar contexto/destino
                                        |
                                        v
                      build solo si hace falta [D13] -> LXC PVE + Quadlets
                                        |
                                        v
                      verificar servicio -> estado a app

NO: navegador -> Named Pipe Windows directamente
NO: carpeta -> shell SYSTEM / root WSL sin admisión
NO: copiar runtimes o workloads del legacy como dependencia del despliegue
```

El flujo de despliegue es requisito; un motor genérico de pipelines no lo es.
Una vez instalado, el servicio lo gobierna su Quadlet/systemd en el LXC; GnX no
necesita otra API `workloads start/stop` ni bucles propios de recuperación.
Definir dónde se construye, si crea o selecciona LXC y cómo se accede al servicio
en `D13`; no inventar un registry, catálogo, CI ni dominios por servicio.
Ambas entradas usan la misma admisión y aplicación, no dos instaladores. Un
path de navegador no concede acceso al disco del host: la web entrega archivos;
la CLI puede resolver un path local con autorización del usuario.
El receptor/puente concreto es una decisión, no un quinto contenedor base ya
autorizado. No trasladar claves del nodo a la carpeta ni al navegador.

Quadlets y `Containerfile` pueden ordenar ejecución/mounts: validar nombres/hashes
no los hace seguros. Rechazar escapes de ruta, enlaces peligrosos y archivos fuera
del contexto; limitar tamaño, recursos, tiempo, red y privilegios. Nunca montar
el pipe, socket Podman del nodo ni credenciales administrativas dentro del build.
Aplicar solo al LXC autorizado, con estado/ownership y recuperación ante fallo;
no modificar los cuatro servicios base al desplegar una aplicación.

### Primer caso real: un LXC, tres aplicaciones

```text
WSL: Tailscale + Pi-hole + Caddy + PVE       núcleo, sin duplicarlo en el guest
                                  |
                                  +-- LXC de servicios
                                        +-- pocket-id [IdP]
                                        |      +-- OIDC --> NetBird
                                        |      +-- OIDC --> RustFS
                                        +-- netbird   [red de sus clientes]
                                        +-- rustfs    [almacenamiento]
```

Son servicios desplegados mediante los mismos Quadlets del usuario, no ramas
`install-rustfs/pocket/netbird` en Rust ni dependencias del arranque de app.
Reutilizar Caddy/CA/Pi-hole del nodo para acceso acordado; no añadir otro gateway,
DNS, sidecar Tailscale por guest ni sustituir Tailscale por NetBird. Pocket ID
no autentica `app.gnx`, PVE ni la CLI. Cada aplicación conserva sus datos/secretos;
Pocket no necesita almacenar su propia base en RustFS para este caso.

Configuración/API oficial primero; GUI mediante la skill adecuada solo si falta
una vía programática soportada, con el mismo estado final verificable. Dos
clientes OIDC separados, issuer/callbacks exactos, CA confiada desde navegador y
contenedores, permisos/grupos mínimos y secretos protegidos. RustFS: validar
login de Console y autorización IAM; SSO no equivale a credenciales S3 universales.
NetBird: comprobar protocolos reales, no solo su login; gRPC/WebSocket y STUN
UDP no se sustituyen con un proxy HTTPS genérico. No abrir puertos públicos por
seguir un quickstart. Passkey/consentimiento personal queda explícitamente pendiente
si requiere al usuario, sin simular aceptación ni bloquear el núcleo.

**Cabo abierto D14:** tres aplicaciones no garantizan tres contenedores. El OCI
oficial NetBird combined no incluye dashboard; con UI necesita otro contenedor
en el mismo LXC. Acordar dashboard o administración sin él; no ocultar un cuarto
servicio ni reconstruir una imagen multiproceso solo para cumplir un contador.
Fijar/verificar versiones con OIDC nativo antes de dar la integración por hecha.

## 5. WSL y Quadlets, sin requisitos inventados

```text
Windows virtualiza WSL2             NECESARIO
  +-- kernel Linux + systemd
       +-- contenedor Proxmox
            +-- interfaz web       NO necesita KVM
            +-- LXC                NO necesita KVM
            +-- VM QEMU acelerada  SÍ necesita KVM [fuera del núcleo propuesto]
```

`nesting` de contenedores no equivale a virtualización anidada de hardware.
Modo LXC sin KVM y elección de imagen: pendientes `D02`/`D05`. Verificar el
entrypoint **del digest elegido**; no trasladar opciones de otra imagen.
No eliminar la virtualización que WSL2 sí necesita ni fingir un check KVM verde.

- Importar una base WSL verificable bajo el perfil dedicado; `systemd=true`.
  La distro/base, WSL/Podman soportados y recursos mínimos se fijan en `D07`.
- Quadlets concretos `.container`/`.network` y units solo donde correspondan;
  archivos externos revisables, sin `.in` ni shell incrustado en Rust.
- Imágenes por digest y archivos por hash. Sin `latest`, upgrades espontáneos
  ni builds OCI al arrancar el usuario.
- Dependencias de disponibilidad expresadas en systemd; salud real, timeouts,
  backoff y límites de reinicio. No `sleep` como prueba ni polling infinito.
  El acceso local Caddy/Pi-hole no tiene `Requires/BindsTo` de mesh enrolada,
  de su salud ni de compute. El target agrupa lifecycle, no exige un "todo sano"
  para servir app; autenticación remota pendiente no bloquea el bootstrap local.
- Volúmenes persistentes separados de contenedores/configuración aplicada;
  re-crear un contenedor no implica re-crear datos, CA ni identidad.
- Rootless no es un dogma: inspeccionar los requisitos reales de Proxmox.
  Cada privilegio, device, mount o capacidad debe justificarse; no privilegiar
  mesh/DNS/gateway porque compute lo requiera. No añadir hardening que rompa
  systemd/cgroups/LXC sin comprobarlo.

## 6. Lifecycle nativo, no framework de workloads

```text
SCM Windows -> recupera el servicio Rust
                         |
gnx -> pipe -> servicio -> WSL de gnxnodesvc -> systemd
                                               |
                                  target GnX (nombre propuesto)
                                    +-- mesh.service     [Quadlet]
                                    +-- dns.service      [Quadlet / Pi-hole]
                                    +-- gateway.service  [Quadlet / Caddy]
                                    +-- compute.service  [Quadlet / PVE]
                                                            |
                                                        PVE gestiona LXC
                                                            |
                                                   systemd/Quadlets del LXC
```

- `gnx start` y `gnx stop` están confirmados para el nodo (`C17`); se traducen
  a iniciar/detener el target propio, mediante el pipe y la identidad dedicada.
  No recorrer una lista de workloads ni llamar `podman stop` por cada servicio.
- Declarar propagación y orden de parada con systemd: `Wants/Requires` para
  arrancar no bastan para propagar Stop; usar relaciones como `PartOf` donde
  corresponda. Respetar parada inversa de dependencias y timeouts.
- Parar PVE de forma ordenada mediante su systemd; su gestión nativa de guests
  debe apagar LXC antes de terminar el contenedor. Verificar señales/timeout y
  ese comportamiento en la imagen elegida; no sustituirlo por kill ni otro supervisor.
- SCM recupera el servicio Windows; systemd recupera contenedores; PVE gestiona
  LXC. Rust solo sostiene la frontera Windows/WSL y el acceso host. No duplicar
  sus políticas de restart, health ni autostart.
- Conservar solo ownership, operación/configuración aplicada e intención mínima
  de arranque del nodo; no una máquina de estados por workload. Una parada
  explícita no debe ser deshecha por la recuperación del servicio Windows.

### CLI: iniciar y detener el nodo

| Comando | Contrato |
| --- | --- |
| `gnx start` | Inicia el núcleo ya preparado mediante su target systemd. Conserva datos, CA, credenciales e identidad mesh; no instala, reenrola ni aplica una actualización pendiente. Si falta preparación, informa la causa/acción sin convertir start en setup. |
| `gnx stop` | Detiene ordenadamente el target propio y sus servicios, incluidos los LXC mediante la parada nativa de PVE. Conserva datos e intención de parada; no elimina recursos ni ejecuta `wsl --shutdown`. |
| `gnx status` | Informa la parada o el avance/resultado observado sin arrancar el nodo. No presenta un dato anterior como salud actual. Permisos de lectura pendientes en D08. |

No hay grupo `workload/workloads`, aliases legacy ni argumentos para elegir
aplicaciones. El servicio Windows sigue disponible para recibir `start`;
su recuperación no deshace `stop`. Repetir start/stop sobre el estado ya alcanzado
no reinstala ni recrea datos. Confirmar el resultado nativo, no solo el envío de
la orden; timeout no equivale a parada o arranque completados.

Una parada explícita retira también el acceso a `app.gnx`/`compute.gnx`;
la CLI informa mientras están detenidas. Esto no contradice C14: una key pendiente
o un fallo mesh no deben detener la app, pero `gnx stop` sí detiene el núcleo por
petición del usuario. No prometer liberar toda la memoria ni apagar WSL ajenas.
Al iniciar, separar disponibilidad local, mesh y compute; no exigir enrolamiento
para recuperar la vista local ni declarar todo sano porque el target está activo.

Estado público derivado y sanitizado; no otra fuente de verdad. `status` muestra
estado nativo y antigüedad; no importa distros, pide claves, descarga imágenes ni
despierta WSL detenida para "consultarla". Arranque sin login y recuperación tras
caída deben demostrarse, no asumirse. No equiparar activo con HTTPS sano.
Timeout significa desconocido/fallo de operación, no permiso para repetir una
mutación a ciegas. Desconectar CLI no duplica ni cancela implícitamente el trabajo.
Reportar por separado **acceso local**, **enrolamiento/conectividad mesh** y
**compute**. "App local disponible; mesh pendiente" es un resultado válido,
no instalación fallida ni nodo remoto aceptado. No crear un framework de estados
para representarlo: observaciones nativas + avance de la operación vigente bastan.
No hay bloqueos globales: una aplicación/key/configuración pendiente afecta solo
su operación dependiente, mientras app conserva avance/resultado/acción. Esto
no autoriza saltar ownership, autenticación o validación para fingir éxito.

Stop afecta solo recursos propios; **no `wsl --shutdown` global**. Reanudar no
reinscribe mesh. Retry no resetea contraseña, no sustituye datos y no aplica una
actualización disruptiva encubierta. Reiniciar el host exige consentimiento.

## 7. Empaquetado y entrega acotada

```text
Rust + configuración + scripts + inputs verificados
                        |
                        v
              build -> MSI -> EXE con MSI incluido
                        |
                        v
          instalar -> configurar por gnx -> comprobar ambas URLs
```

Propuesta `D10`: WiX + Burn para conservar MSI y EXE sin escribir un instalador
propio. Toolchain .NET solo de build, no runtime del producto; versión/licencia/
condiciones FOSS por verificar. No investigar alternativas sin un impedimento
concreto. Instalación silenciosa no pregunta secretos ni abre onboarding.

**Autocontenido/offline es requisito confirmado (`C13`), no otro lifecycle.**
Contenido lógico del instalable; nombres internos por fijar:

```text
EXE
 +-- MSI completo (también instalable offline por separado)
      +-- bin/          CLI y servicio Rust
      +-- filesystems/  rootfs WSL con systemd, Podman y dependencias
      |                 base LXC preparada para el flujo final que la necesite
      +-- images/       OCI locales: Tailscale, PVE, Caddy y Pi-hole
      |                 inputs del caso LXC incluidos, sin activar aplicaciones
      +-- config/       Quadlets, WSL, DNS, gateway y configuración Windows
      +-- scripts/      aplicación/instalación mínima .run/.sh
      +-- manifest      versiones, arquitectura, origen y hashes
```

No `apt`, `pull`, `pveam download` ni descargas de dependencias ocultas durante
instalación. Preparar los inputs en build, verificar al instalar y usar
`wsl --import`/carga OCI nativos, sin framework importador. Si falta un input,
informar el paquete incompleto; no recurrir silenciosamente a Internet.

Decidir archivo OCI o store pre-cargado por tamaño y fidelidad del digest;
no duplicar las imágenes en ambos. Sin credenciales, CA privada, identidad mesh,
machine-id ya inicializado ni estado de PVE clonados. Las bases de archivos son
plantillas limpias, no respaldos de una instancia operativa. Llevar una base LXC
no significa crear una aplicación de fábrica.
Los OCI/dependencias del caso RustFS/Pocket ID/NetBird también se entregan locales
para su despliegue offline declarado, separados de las cuatro capacidades base.
No confundir llevar sus artefactos con provisionar el LXC durante el bootstrap.

Rootfs nuevo sirve para primera importación, no para reemplazar una WSL existente
al actualizar. Medir espacio temporal/expandido y comprobar instalación sin red.
Windows/WSL2 soportados y redistribución de sus prerrequisitos siguen en `D07`/
`D10`; no prometer cualquier Windows limpio. Enrolar Tailscale requiere acceso
a su plano de control. Construir una aplicación futura offline exige que su
contexto aporte también sus bases/dependencias: no hay caché universal implícita.

Separar transacción MSI del provisioning Linux. Upgrade conserva identidad,
WSL, datos, certificados y credenciales; activación disruptiva con confirmación.
Desinstalación/purga y convivencia con GnX legacy: `D11`, sin destrucción implícita.
Ni la instalación "limpia" ni Repair autorizan limpiar datos anteriores.

Layout futuro, no archivos que debamos crear ahora:

```text
apps/          CLI Rust + servicio Rust
crates/        protocolo compartido si evita duplicación real
config/        archivos finales: Windows, WSL, Quadlets, gateway, DNS
scripts/       instalación/aplicación fija .run/.sh; build Windows mínimo
installer/     definición MSI/EXE
```

CLI silenciosa por defecto: resultado + causa/acción si falla; progreso solo al
cambiar de fase. Diagnóstico ampliado explícito y sanitizado; retención/tamaños
acotados. Dependencias mínimas, sin framework web ni telemetría externa.

**Aceptación del núcleo:** instalar sin descargas en host autorizado y comprobar
app local con TLS válido y avance **antes** de introducir la key mesh. Configurar
secretos por CLI, abrir ambas URLs y comprobar salud real y recuperación/stop sin
pérdida ni interferencia con WSL ajenas. Pruebas pequeñas de fronteras críticas;
no batería grande ni workloads de demostración. Key inválida, falta de Internet
o caída de mesh no deben retirar app local; no declarar acceso remoto hasta verificarlo.

**Aceptación final adicional:** entregar por app o CLI Quadlets/contexto aprobados,
desplegar realmente el caso de un LXC y comprobar login Pocket ID en NetBird y
RustFS, persistencia y salud de servicios; devolver estado verificable sin
exponer secretos ni ampliar privilegios del nodo. No afirmar esta capacidad por
tener un selector de carpeta o por reutilizar el importador legacy.

**Trabajo lean:** bloques de hasta 20 minutos con un hito acotado, resultado
observable y siguiente paso preciso. No es un SLA de producto completo ni de
login humano. Reanudar desde archivos/configuración/estado real, no empezar otra
arquitectura ni resetear la instalación. Un impedimento local no paraliza trabajo
independiente autorizado. En esta fase el resultado sigue siendo documental.
**Hoy: solo los tres documentos acordados; sin implementación.**
