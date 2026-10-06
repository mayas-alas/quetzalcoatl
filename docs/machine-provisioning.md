# Máquinas dedicadas y publicación de servicios

Estado: alcance del siguiente update, **no implementación desplegada**.
La release instalada sigue siendo v0.1.72. No aplicar este feature como hotfix.

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
| `private-services` | RustFS, Pocket ID, NetBird management/server/dashboard y componentes compartidos actuales |
| `ai` | Voice, cliente mesh y servicios AI explícitamente elegidos |

El proxy de NetBird es infraestructura de acceso, no parte del proceso Voice.
Puede ejecutarse como Quadlet aislado en servicios privados si sólo se usa dentro
de la red. Una entrada pública puede requerir ubicarlo en un host accesible desde
Internet. No crear una tercera máquina sin esa necesidad y decisión de topología.
No abrir puertos públicos ni mover servicios existentes automáticamente.

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

La aceptación del update debe comprobar creación idempotente/ownership de AI,
no modificación de datos del LXC existente, arranque Quadlets, HTTPS con CA,
denegación no autorizada, texto/TTS/audio real, actualización/reinicio, y browser
skill sobre Proxmox y Voice. Un mock, digest de registry o pantalla de login no
sustituye esas pruebas. No declarar este feature construido o instalado todavía.
