# Alcance de GnX Core

## Producto actual

GnX Core instala un nodo privado en Windows x64. Su CLI y servicio Rust mantienen
la frontera de autorización Windows y una WSL dedicada; systemd/Quadlets ejecutan
Tailscale, Pi-hole, Caddy y Proxmox VE. El usuario opera el nodo sin mantener una
terminal como supervisor.

| Área | Incluido |
| --- | --- |
| Entrega | EXE Burn como entrada interactiva recomendada y MSI autocontenido como vía administrativa directa al mismo producto. |
| Automatización | GitHub Action manual para compilar una versión nueva desde fuentes o empaquetar el CLI versionado, entregar MSI/EXE con hashes y, opcionalmente, publicar una pre-release con tag coincidente. |
| Identidad | Servicio `GnXCore`, cuenta `gnxcoresvc`, WSL `gnx-core` y estado protegido propio. |
| Control | `start`, `stop`, `status`, `enroll`, `password` y `deploy <carpeta>` mediante un pipe administrativo local; las fuentes 0.0.20 añaden `update --check|--apply [catalogo.json]`. |
| Acceso | `app.gnx` y `compute.gnx`; el stack privado registrado añade Pocket ID, RustFS/S3 y NetBird por HTTPS. Acceso mesh sujeto a DNS, ACL y confianza del cliente. |
| Certificados | CA persistente, confianza Windows interna, renovación de hoja servidor y CRL mediante systemd. |
| Conservación | Actualización y reinstalación preservan identidad, CA, credenciales y datos. Desinstalación normal sin purga. |

La app es informativa: no recoge credenciales, no recibe archivos y no ejecuta
operaciones administrativas. `status` observa el target, no un diagnóstico global
de disponibilidad. Una app accesible no demuestra que mesh o compute estén listos.

El MSI es el único dueño de la transacción de instalación Windows. El EXE delega
en él, sin otro provisioning ni lógica de retirada del nodo. El servicio prepara
y opera el runtime mediante sus fronteras existentes. La release para usuarios
contiene solo MSI, setup EXE y sus hashes; CLI, manifiesto y digest base son
artefactos técnicos de CI, no descargas adicionales para instalar.

## Despliegue de Quadlets

Admisión de una carpeta de Quadlets/configuración
por la CLI, transferencia por el pipe existente y creación de un LXC nuevo con
`pct` desde Rust. Aplicación y lifecycle mediante Podman/systemd del guest, sin
motor de workloads ni ejecución de scripts aportados por el usuario en el nodo.
No se presenta el envío web como terminado por ello.

Desde 0.0.16, `gnx deploy <carpeta>` permite enviar
Quadlets/configuración a un LXC nuevo. La admisión, recursos, propiedad y límites
están en [Despliegue](docs/deployment.md). La carpeta debe estar bajo control del
administrador, no en una ubicación compartida modificable por terceros.

La candidata 0.0.18 admite `.container` y `.network`, verifica cada unidad e incluye
el caso fijo [stack privado](stacks/private-services/README.md): RustFS, Pocket ID
y NetBird con dashboard, red bridge, publicación HTTPS condicional e integración
OIDC mediante las APIs nativas. Conserva datos, CA y credenciales; no añade un
catálogo, otro supervisor ni imágenes reconstruidas. El alta humana usa la passkey
real del operador. No se acredita login ni conectividad entre peers por configurar OIDC.
El arranque compute espera disponibilidad de PVE mediante su healthcheck nativo
antes de reconciliar la publicación del stack.

No existen todavía subida web, build por Containerfile ni soporte de todas las
clases de Quadlet (`.volume`, `.pod`, `.build`). La publicación no es genérica:
solo el stack privado registrado tiene los dominios y rutas fijos del producto.
La admisión no valida toda la semántica de Quadlet ni todas las referencias de
configuración antes de crear. Un recibo completado no es una comprobación de salud.
En 0.0.16, el cierre agregado tampoco confirma que todas las unidades estén activas.

La presencia de Proxmox no acredita por sí sola despliegues ni VMs aceleradas.
No se incluyen aplicaciones de negocio en el núcleo ni se crean guests al instalar.

Las fuentes admiten seguimiento explícito de tags de aplicaciones con
`Pull=newer` en [la política de imagen](docs/deployment.md#política-de-imagen-al-arrancar).
La descarga queda a cargo de Podman al crear cada contenedor, sin otro supervisor
ni timer. No selecciona canales, migra guests existentes ni activa actualizaciones
del núcleo: este conserva sus imágenes offline fijadas.

## Requisitos pendientes autorizados

Las fuentes 0.0.20 implementan la [comparación y aplicación explícita](docs/updates.md)
de un catálogo administrativo por digest sobre las ocho unidades conocidas, sin
recrear el stack. Incluyen la consola oficial RustFS v0.1.38 como recurso offline.
La comparación al iniciar es local y no aplica cambios. Esto no cierra descubrimiento
remoto, selección automática de versiones compatibles ni aceptación runtime del
actualizador: dependen de una entrega instalada y comprobaciones independientes.

- La priorización de actualizaciones y los nuevos frontends se registran en
  [features.md](features.md): **OpenUI** es el estándar; **Appless** es referencia
  para clientes móviles. No son capacidades ya incorporadas a `app.gnx`.
- **R-IMG-01 — Imágenes al iniciar:** extender la actualización al arranque tanto
  a aplicaciones como a los cuatro servicios del núcleo, con canales explícitos,
  semilla offline verificada e intención de parada respetada. No incluye actualizar
  automáticamente el CLI/MSI ni reemplazar el filesystem WSL.
- **R-LXC-01 — Base recuperable:** disponer de una base bootc versionada y
  recreable para los LXC, con retorno a la base anterior si una actualización
  falla. Verificar primero qué mecanismo bootc es compatible con el LXC no
  privilegiado; exportar un rootfs desde una imagen bootc no demuestra rollback
  transaccional dentro del guest. Conservar propiedad, identidad y datos; separar
  el rollback de la base del de imágenes Podman y de migraciones de datos.
- **R-UPD-01 — Actualización explícita del producto:** unificar en
  `gnx update --apply` la selección de una release pública, incluidos candidatos
  publicados, su verificación, la actualización nativa Windows y la aplicación
  explícita del catálogo de componentes. El origen y nombre del MSI se configuran
  administrativamente mediante `GNX_UPDATE_REPOSITORY` y `GNX_UPDATE_MSI_ASSET`
  de ámbito máquina. Un origen privado o inaccesible falla sin instalar,
  solicitar credenciales ni cambiar de repositorio. Conservar MSI/Burn como
  dueños de instalación y mantenimiento; no reemplazar el servicio desde su
  propia operación bloqueante ni reiniciar el host o detener guests implícitamente.
- **R-REC-01 — Actualización o recreación explícita:** sin fuerza,
  `gnx update --apply` actualiza imágenes conservando volúmenes, identidad y
  configuración declarada; los LXC del usuario y del sistema siguen disponibles.
  La forma prevista `gnx update --apply --force` vuelve a preparar el
  sistema GnX desde cero, retirando su estado persistente e identidades anteriores
  y aplicando el bootstrap configurado. El borrado del almacenamiento administrado
  incluye sus guests, también los creados por el usuario. No significa reiniciar
  Windows, purgar otras WSL ni retirar recursos externos al nodo. Antes de ejecutar,
  el inventario debe mostrar los datos y guests afectados y la autorización cubrir
  su pérdida; no se adopta por ello el control de guests ajenos en otros recorridos.
  La implementación debe delegar en mecanismos nativos, sin omitir integridad,
  ownership o control de resultados desconocidos. Ninguno de los flags
  `--force`, `--force-recreate` o `--remove-orphans` existe hoy en esa CLI.
- **R-VAL-01 — Aceptación manual recreada y compartida:** distinguir la
  preparación de un entorno nuevo de producto y su operación posterior por
  humano y agente sobre la misma instancia. Mantener `gnx password` como vía
  explícita de configuración y cambio de la credencial PVE, con entrada y
  confirmación sin eco y persistencia del hash protegido. No añadir otro gestor
  de contraseñas, una clave global Windows ni recuperación de secretos.
  `PASSWORD`/`PASSWORD_HASH` declarados son la fuente de verdad del
  entrypoint: al recrear el contenedor, incluso durante una actualización de
  compute sin fuerza, se reaplica esa credencial. Cambiar root solo dentro del
  contenedor no modifica la configuración persistente. No añadir sincronización
  de contraseñas ni prometer que ese cambio local sobreviva a la recreación.
  Un cambio persistente se realiza con `gnx password`; revisar su UX diferenciando
  nueva contraseña de `root@pam`, credenciales Windows, confirmación, errores
  previos a la aplicación y resultados desconocidos.
  Conservar privada la identidad Windows de servicio
  y usar CLI/pipe como recorrido normal del agente. El acceso nativo PVE/Podman
  se acota al entorno y recursos autorizados. Registrar fuera de Git la ficha
  de ejecución y contrastarla al retomar; no introduce ejecución de pruebas
  automáticas, simulaciones ni un segundo nodo paralelo en el mismo Windows.

R-UPD-01, R-REC-01 y R-VAL-01 son objetivos del trabajo para 0.0.23, no capacidades
ya incorporadas ni aceptadas. La recreación no es un requisito de cada actualización
y no se usa para simular que se conservó estado durante un upgrade.

Sus [criterios de cierre](historias-de-usuario.md#requisitos-pendientes) no son
funcionalidad implementada ni aceptación. No autorizan recrear un guest existente
ni aplicar una nueva política a la instalación sin confirmación separada.

## Límites de seguridad

- Coincidir en nombre no demuestra propiedad: verificar SID, registro WSL,
  rutas y recibos antes de reconciliar o retirar recursos.
- No adoptar recursos existentes ni modificar WSL, cuentas, datos o configuración
  ajenos. No usar `wsl --shutdown`, prune global o purga implícita.
- Secretos por entrada sin eco y pipes protegidos. Nunca por argv, URL, Git,
  estado público o logs. DPAPI no sustituye las ACL.
- Sin shell libre, API HTTP privilegiada, plugins ni supervisor duplicado.
- La app local no depende de la key mesh ni de compute. Un `stop` explícito sí
  detiene el acceso y debe respetarse durante la recuperación.
- No desactivar TLS, revocación ni protecciones Windows para obtener un resultado
  favorable. No cambiar CA, red o políticas externas sin autorización específica.

## Entrega y verificación

Los inputs offline se preparan fuera del checkout y no contienen identidades ni
secretos de una instancia. El contrato de distribución y la Action están definidos
en [Empaquetado](docs/packaging.md). No se prometen descargas o releases publicadas
antes de que existan. El workflow no instala el producto ni acredita su aceptación.

La compilación, el empaquetado, la instalación, la configuración aplicada y la
aceptación son resultados distintos. La comprobación realizada en un host no
demuestra instalación en Windows limpio, recuperación tras reboot, vencimiento
futuro de certificados, login humano Proxmox ni acceso desde dispositivos externos.
Una sesión Proxmox ya abierta tampoco demuestra una autenticación nueva.
Desinstalación, rollback y reinstalación requieren comprobar su ciclo propio;
revisar su registro Windows o sus acciones MSI no equivale a ejecutarlo.

## Fuera de alcance

Purga genérica de datos; migración automática de otras instalaciones;
actualización automática del CLI/MSI sin petición administrativa y actualización
del filesystem WSL;
multiusuario/multinodo; catálogo de aplicaciones; motor genérico de pipelines;
otro backend de control; IdP para la app; telemetría remota; cambios BIOS, TPM,
Secure Boot, BitLocker o antivirus. La Action de instaladores no es un motor de
despliegue del producto ni autoriza publicar una release.

## Autoridad documental

1. La petición vigente del usuario delimita el trabajo autorizado.
2. Este documento fija el alcance y separa lo disponible de lo pendiente.
3. [Arquitectura](architecture.md) describe los mecanismos implementados.
4. [Recorridos](historias-de-usuario.md) define resultados esperados, no evidencia
   de aceptación ni permiso para implementar funciones adicionales.
5. [`.AGENTS`](.AGENTS) establece la disciplina de ejecución de agentes.

Resolver contradicciones antes de modificar la parte afectada. No convertir una
propuesta, un criterio o una referencia histórica en autorización.
