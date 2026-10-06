# Máquinas dedicadas y publicación de servicios

Estado: implementación parcial de enrolamiento BYOP, **no feature desplegado**.
La release instalada sigue siendo v0.1.72. No aplicar este feature como hotfix.
El cliente BYOP tiene pruebas aisladas; falta el provisioner/stack AI y su build/aceptación.

## Objetivo

Crear programáticamente una máquina LXC dedicada a AI, separada de la máquina
existente de servicios privados. Usar perfiles declarativos, Quadlets externos,
imágenes fijadas, persistencia y límites de red, sin shell arbitrario desde la
PWA. Incorporar el proxy oficial de NetBird como componente de publicación.

No usar niveles de negocio, sufijos generacionales o nombres de feature en
máquinas, servicios, paths o archivos. Los VMID de Proxmox son identificadores
internos, no nombres de negocio. Las versiones pertenecen a metadata/releases,
no a nombres de máquinas.

## Inspección realizada

- Código actual: `payload/node/compute/lxc.run` sólo permite el workload `rustfs`.
  Busca el tag `gnx-rustfs`, crea hostname `rustfs` y suministra la máquina
  compartida a `/opt/gnx/workloads/rustfs`. Ese nombre no representa su alcance.
- Browser skill, Firefox/Proxmox: observado **LXC 100 (rustfs)** en ejecución,
  tags `gnx`/`gnx-rustfs`. No se renombró ni recreó durante esta inspección.
- Browser skill, NetBird DNS: observado el dashboard Nameservers y su mensaje
  de que todavía no hay nameservers. Agregar un nameserver no publica un proxy
  ni crea automáticamente una ruta o peer.
- `compute.container` ya monta directorios persistentes de datos, configuración,
  certificados públicos y red. No incluye la clave firmante de la CA.
- Voice publicado: repo HF `Maya-alas/gnx-voice-agent`, revisión
  `db004950a09cf907471d1e30565a73837b801c48`, versión declarada v0.0.2.
  El checkout local tiene cambios sin commit para TTS Hugging Face y secretos
  por archivo. No se sobrescribieron ni se asumieron publicados esos cambios.
- `npm test` del checkout Voice: **8 tests pass**, providers/API/MCP con mocks.
  No prueban inferencia pagada, audio real ni una imagen OCI desplegada.

Esta evidencia es de árbol de accesibilidad/inspección de código; no es una
captura de una máquina AI ya creada. No se usaron archivos temporales de trabajo.

## Separación propuesta

| Máquina | Responsabilidad |
| --- | --- |
| `private-services` | RustFS, Pocket ID, NetBird management/server/dashboard y Continuity `business` actuales |
| `ai` | Voice, Oryx, OmniRoute, Continuity `ai` y el proxy oficial NetBird BYOP como Quadlets independientes |

Alcance ampliado por la persona operadora: usar el NetBird existente para vincular
el proxy del nuevo LXC a un cluster de su cuenta, sin duplicar management ni
dashboard. El proxy es infraestructura de acceso, no parte del proceso Voice.
El dashboard inspeccionado muestra **No clusters available**; tener management
activo no equivale a tener un cluster BYOP registrado. El update deberá registrar
el cluster en la cuenta existente y comprobar su conexión antes de afirmar ready.
Una entrada pública todavía requiere dominio y ubicación alcanzable; no abrir
puertos públicos, crear otra máquina ni mover servicios actuales implícitamente.

Voice consume servicios remotos de inferencia/Hugging Face y las APIs necesarias
de Oryx y OmniRoute. Las rutas y modelos se fijan en configuración externa. No
suponer que Oryx ofrece un endpoint genérico TTS compatible con el adapter Voice:
validar el contrato upstream y ejecutar una prueba real de audio.

La migración de nombre del LXC existente debe conservar VMID, disco/rootfs,
cuenta, identidades de red, claves y datos. Hostname/tag y paths que representan
la **máquina compartida** pasan a `private-services`; nombres de servicios reales
como `rustfs.container`, `rustfs.gnx` y los datos de RustFS siguen siendo correctos.
No hacer un reemplazo textual de `rustfs` indiscriminado ni mantener dos árboles
activos. Resolver ownership y colisiones antes de migrar, con receipt idempotente.
Los aliases IPC existentes deben retirarse o migrarse con un contrato versionado,
no cambiar su significado bajo la misma versión del protocolo.

## Perfil mínimo AI

- LXC sin privilegios; Debian fijado por checksum, Podman/systemd/Quadlet.
- CPU/RAM/disco explícitos y ajustables; sin GPU/modelos locales por defecto.
  Voice actual compone inferencia remota y MCP, no requiere un stack de GPU.
- Aplicación con UID no root, capacidades mínimas y filesystem de imagen readonly.
- Cliente de red con identidad y estado propios; TUN/capacidades sólo donde haga
  falta. No copiar la identidad de la máquina compartida ni usar privileged global.
- Archivos de configuración, Quadlets y datos en directorios persistentes
  nominales `ai`, separados del almacenamiento de otras máquinas.
- Secretos por archivos/secret stores con ownership estricto. No reutilizar la
  cuenta/token HF de publicación como credencial de inferencia implícitamente.
- Acceso a RustFS mediante API S3 y credencial limitada al bucket/prefijo necesario;
  no montar todo el disco de servicios privados ni compartir su clave admin.
- Montar la CA **pública** para validar dependencias y certificados leaf separados
  para los nombres de servicio. Nunca distribuir la clave firmante de la CA.

Perfiles externos bajo `payload/node/workloads/`; un registro de ownership enlaza
nombre lógico, VMID, perfil y estado aplicado. VMID asignado por Proxmox, nunca
`100 + índice`. Rechazar nombres/tags duplicados y guests no gestionados. Crear,
aplicar, arrancar y detener son operaciones allowlisted del agente/CLI elevada;
la PWA sólo muestra estado y enlaces, sin aceptar manifiestos o comandos root.
Las futuras invocaciones deben ser fáciles, pero no anunciar una sintaxis CLI
como disponible antes de implementarla y probarla.

## Volúmenes y actualización

Bind mounts hacen visibles archivos, **no prometen hot reload**. Montar un archivo
individual puede conservar el inode anterior si el host lo sustituye por rename;
preferir directorios readonly para configuración publicada atómicamente.

El reconciliador debe validar primero, suministrar desde el payload versionado,
ejecutar daemon-reload, recargar cuando el proceso lo soporte y reiniciar sólo
las unidades afectadas cuando lo necesite. Registrar el digest/config aplicada
únicamente después de health real. Cambiar el runtime en vivo no sustituye un
nuevo MSI/Burn, su instalación y su aceptación. Builds/evidencia son salidas
versionadas/durables; no dejar runners o árboles scratch en el producto.

## Proxy oficial de NetBird

Upstream comprobado: [NetBird v0.80.0](https://github.com/netbirdio/netbird/releases/tag/v0.80.0),
publicada 2026-10-01. La definición de release publica
`netbirdio/reverse-proxy` y `ghcr.io/netbirdio/reverse-proxy`.

Manifest GHCR consultado en esta inspección:

- Tag de referencia: `ghcr.io/netbirdio/reverse-proxy:0.80.0`.
- Índice multiarch: `sha256:6d6655b3f13f837d80cc44878033866713932d3c5c1381ed869af9542b523bc5`.
- Manifest Linux amd64: `sha256:735c68a60624c72c315e7c3612bfe760f8c75251cf327afd2b5e0a60c9cdab6e`.
- Licencia del componente: AGPLv3; conservar avisos/fuente upstream.

Esto verifica metadata del registry, **no un import OCI offline, arranque del
binario o compatibilidad con el management instalado**. Antes de activarlo:

1. Verificar conjuntamente versiones de management/dashboard/proxy y sus APIs.
2. Incluir su imagen exacta en el bundle offline, verificar digest importado y
   usar Quadlet/Pull=never. No usar latest ni descargas de runtime implícitas.
3. Configurar identidad del proxy, token propio, storage de certificados y
   rutas/policies explícitas. No crear un servicio público sin autenticación.
4. Basarse en el CLI de la versión fijada, no sólo en su README: v0.80.0 usa
   `NB_PROXY_TOKEN`, `NB_PROXY_DOMAIN`, `NB_PROXY_MANAGEMENT_ADDRESS` y flags de
   certificados. El README conserva ejemplos más antiguos; no inventar opciones
   `*_FILE` ni meter credenciales en argv/Git/browser/logs.
5. Mantener Caddy para la ruta privada actual. Evitar colisión de listeners TLS y
   tráfico de auth; no sustituir el login app.gnx con la incorporación del proxy.

### BYOP con nuestro Caddy y custom domains

Requisito confirmado por la persona operadora: Caddy conserva la entrada HTTPS;
el contenedor oficial NetBird se vincula a un **account cluster**, no a un cluster
shared mediante el `authSecret` global del management. Caddy no implementa ese
registro gRPC y no sustituye la imagen oficial.

Topología de entrada propuesta: cliente → Caddy HTTPS → proxy NetBird HTTPS
verificado → backend del servicio. Listeners separados, Host original conservado,
headers forwarded saneados y confianza sólo en el frontend esperado. Caddy puede
terminar TLS, pero no desactivar la validación del backend. Si Caddy ocupa 443,
no dejar ACME tls-alpn-01 del proxy esperando un challenge que Caddy no le entrega:
usar certificados leaf/static propios o una estrategia explícita compatible.
Nunca compartir la clave firmante de la CA.

- `NB_PROXY_DOMAIN` define el apex del cluster. El dominio integrado self-hosted
  es `{subdomain}.{cluster-apex}`; no tiene el nonce/domino cloud de NetBird.
- Los custom domains se registran contra ese cluster y requieren CNAME verificable.
  Una entrada Caddy o DNS A sola no acredita ownership ante NetBird.
- El DNS debe cubrir el apex, los subdominios de servicios y la verificación del
  custom domain. Comprobar contra el resolver que realmente usa management.
  La implementación v0.80.0 consulta `validation.<custom-domain>`; el wildcard
  CNAME descrito en la guía debe resolver también ese nombre.
- Sólo declarar el cluster conectado cuando su API indica `type=account`,
  `online=true` y `connected_proxies>0`; comprobar además health del proceso y
  `management_connected`/`initial_sync_complete` antes de afirmar data-plane listo.
- Un custom domain debe figurar `validated=true` con `target_cluster` correcto.
  No alterar directamente DB/flags ni registrar un dominio sin DNS listo: los
  pendientes caducan a las 48 horas según la guía actual.
- `.gnx` conserva su carácter privado y usa la CA privada; no equivale a un dominio
  público registrado ni habilita acceso desde Internet. Falta confirmar si se
  usarán nombres privados o cuáles son los dominios registrados bajo control del
  operador. No inventar ese dominio, su DNS público o autorización para abrir puertos.

Código parcial: `payload/node/workloads/ai/proxy-enroll.run`, pensado para ejecutarse
localmente en private-services durante el futuro provisioning. Consume config y
PAT por archivos root `0600`, crea token account-scoped, exige ownership, y verifica
cluster/custom domain. No emite valores, no rota tokens automáticamente ni adopta
registros ajenos. **No está conectado todavía al lifecycle del instalador**, no
se ejecutó contra la API con mutaciones ni crea por sí solo el guest/Quadlets AI.

Las guías verificadas son [Custom Domains](https://docs.netbird.io/manage/reverse-proxy/custom-domains)
y [BYOP / shared y account clusters](https://docs.netbird.io/manage/reverse-proxy/bring-your-own-proxy#shared-and-account-clusters).

Referencias upstream:
[README](https://github.com/netbirdio/netbird/blob/v0.80.0/proxy/README.md),
[CLI real](https://github.com/netbirdio/netbird/blob/v0.80.0/proxy/cmd/proxy/cmd/root.go),
[imágenes de release](https://github.com/netbirdio/netbird/blob/v0.80.0/.goreleaser.yaml).

## DNS, dominios y acceso remoto

`voice.gnx` es un nombre **privado**. Split DNS de NetBird puede resolverlo para
peers inscritos, pero requiere DNS alcanzable, peer/rutas/ACL y confianza de CA;
un nombre configurado no prueba conectividad. No modificar el tailnet Tailscale
existente ni sus credenciales por añadir peers NetBird al workload nuevo.

Para un enlace accesible sin instalar NetBird hace falta un dominio registrado,
un proxy alcanzable desde Internet, DNS A/AAAA/CNAME apropiado, TLS público y
política de acceso. El proxy es la entrada; Voice puede seguir privado detrás
del túnel NetBird. La CA privada del nodo no equivale a un certificado público.
No prometer que un CNAME a `voice.gnx` lo hará accesible desde Internet.

## Continuity, almacenamiento y trazabilidad

Continuity forma parte del nuevo LXC AI. La implementación Rust inspeccionada
(`C:/Users/mayas/continuity`, commit `d5c8886`) fija **un scope por instancia**;
el cliente no puede seleccionar otro scope en cada petición. Por ello:

- Conservar `continuity-business`, scope `business`, spool
  `/var/lib/continuity-business` y prefijo `continuity/business`, sin migrarlos.
- Crear en AI una instancia de **la misma imagen OCI verificada**, con
  `CONTINUITY_SCOPE=ai`, spool `/var/lib/continuity-ai`, bucket existente
  `continuity` y prefijo exclusivo `continuity/ai`. No reconstruir una imagen
  por scope ni mantener dos archivos de imagen equivalentes en el bundle.
  Los almacenes Podman independientes de cada LXC sí necesitan importar las
  capas; deduplicar artefactos no significa compartir sus discos/journals.
- Un único escritor por prefijo. Nunca montar el spool business en AI ni cambiar
  su scope. No arrancar dos escritores con el mismo prefijo S3.
- `CONTINUITY_ACK=remote` y CA pública para HTTPS a RustFS, mediante credencial
  S3 limitada al prefijo AI. No copiar la clave admin actual de RustFS al guest
  nuevo, aunque el Quadlet business inspeccionado utilice ese EnvironmentFile.
- Health real: `/health/ready` debe confirmar materialización y réplica sin
  pending/error. `Up`, `/health/live` o un `200` genérico no prueban durabilidad.
- Integrar las cuatro operaciones reales: `continuity.record` →
  `POST /v1/events`, `continuity.search` → `POST /v1/search`,
  `continuity.get` → `GET /v1/entities/:id` y `continuity.timeline` →
  `GET /v1/timeline`. Son nombres lógicos; Continuity no expone por sí solo
  un servidor MCP. El adapter/registro de herramientas pertenece a la plataforma.
- La plataforma verifica identidad y consentimiento. El modelo no puede
  atribuirse un actor humano ni confirmar hechos por declarar `user:` en JSON.
  No añadir grabación indiscriminada de conversaciones/prompts en el journal.

Continuity aporta contexto y referencias de recursos, no un registry OCI ni
almacenamiento de blobs administrado. Para release/codebase, guardar en S3
artefactos/manifests inmutables direccionados por SHA-256 y referencia al commit
fuente; registrar referencias y hashes en Continuity, no copiar imágenes al
journal. No presentar un bucket como registry compatible con Distribution sin
implementar y probar ese protocolo. Separar permisos de publicación del release
de los permisos del proceso AI. No crear otra instancia `codebase` por defecto:
la partición y el consumidor de trazabilidad deben quedar explícitos primero.

Redis se reutiliza sólo si la dependencia real lo exige y se comprueban ACL,
namespace y persistencia. El Valkey actual de sesiones es efímero y no sirve de
fuente durable del registry ni de Continuity. Oryx upstream incluye Redis en su
imagen: comprobar su configuración externa y aislamiento antes de prometer que
usará el existente; no añadir un segundo Quadlet Redis automáticamente. Continuity
no necesita Redis y OmniRoute usa su almacenamiento SQLite persistente.

Cada aplicación nueva necesita un origen HTTPS privado y un cliente OIDC propio
de Pocket ID, sin compartir cookies/client secret con `app.gnx`: destinos
propuestos `voice.gnx`, `oryx.gnx`, `omniroute.gnx` y `continuity.gnx`. Para apps
sin OIDC nativo, usar el boundary OAuth2 Proxy de la plataforma con callback,
PKCE, nonce, cookies y autorización explícita por app. Los endpoints internos
no deben convertirse en una puerta de entrada sin autenticación. No asignar
personas a grupos automáticamente ni confundir login de navegador con autorización
de llamadas entre servicios. Definir el transporte interno/adapters y sus límites
antes de abrir una API fuera del loopback/red privada del workload.

Evidencia de esta ampliación y límites: [inspección AI/Continuity](validation/ai-services/README.md).
Todavía no hay Quadlets AI instalados ni nueva release del feature.

## Decisiones/gates pendientes antes del build de feature

- Definir si se desea sólo acceso con cliente NetBird o un enlace de navegador
  sin cliente; en este último caso indicar dominio y ubicación de entrada pública.
- Elegir la release Voice publicada o incorporar deliberadamente los cambios
  locales de HF TTS en una nueva release del componente con snapshot/hash fijado.
  No hay imagen OCI Voice publicada/verificada en lo inspeccionado.
- Resolver provisión protegida de credenciales de inferencia y enrolamiento sin
  mostrarlas en chat. Permisos/billing HF no se deducen del login del Hub.
- Diseñar y probar la migración de naming/IPC y la compatibilidad del proxy con
  la versión real del management. Respaldar estado antes de migraciones de DB.
- Verificar/importar una sola imagen Continuity fijada por digest; provisionar
  credencial S3 limitada y probar commit/retry/restore en un prefijo de aceptación,
  separado de `continuity/business` y del journal AI productivo.
- Comprobar la configuración Redis que Oryx admite, la integración Voice/Oryx/
  OmniRoute/Continuity y OAuth de cada origen con las versiones fijadas.
- Publicar la nueva release en el S3 existente con checksum y manifest inmutable;
  verificar descarga y actualización real. Esto es adicional a la publicación
  habitual del installer/fuente en Hugging Face, no prueba de registry OCI.

La aceptación del update debe comprobar creación idempotente/ownership de AI,
no modificación de datos del LXC existente, arranque Quadlets, HTTPS con CA,
denegación no autorizada, texto/TTS/audio real, actualización/reinicio, y browser
skill sobre Proxmox y Voice. Un mock, digest de registry o pantalla de login no
sustituye esas pruebas. No declarar este feature construido o instalado todavía.
