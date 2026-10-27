# Alcance de GnX Core

## Producto actual

GnX Core instala un nodo privado en Windows x64. Su CLI y servicio Rust mantienen
la frontera de autorización Windows y una WSL dedicada; systemd/Quadlets ejecutan
Tailscale, Pi-hole, Caddy y Proxmox VE. El usuario opera el nodo sin mantener una
terminal como supervisor.

| Área | Incluido |
| --- | --- |
| Entrega | EXE Burn como entrada interactiva recomendada y MSI autocontenido como vía administrativa directa al mismo producto. |
| Automatización | GitHub Action manual para empaquetar el CLI versionado, entregar artefactos MSI/EXE con hashes y, opcionalmente, publicar una pre-release con tag coincidente. |
| Identidad | Servicio `GnXCore`, cuenta `gnxcoresvc`, WSL `gnx-core` y estado protegido propio. |
| Control | `start`, `stop`, `status`, `enroll`, `password` y `deploy <carpeta>` mediante un pipe administrativo local. |
| Acceso | `app.gnx` y `compute.gnx`, HTTPS local y acceso mesh sujeto a DNS, ACL y confianza del cliente. |
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
No se presenta el envío web o la integración de aplicaciones como terminados por ello.

Desde 0.0.16, `gnx deploy <carpeta>` permite enviar
Quadlets/configuración a un LXC nuevo. La admisión, recursos, propiedad y límites
están en [Despliegue](docs/deployment.md). La carpeta debe estar bajo control del
administrador, no en una ubicación compartida modificable por terceros.

No existen todavía subida de archivos desde la web, build de imágenes, publicación
de dominios de aplicaciones ni integración OIDC de servicios. Esas capacidades
no se presentan como completadas al crear el LXC.

La presencia de Proxmox no acredita por sí sola despliegues ni VMs aceleradas.
No se incluyen aplicaciones de negocio en el núcleo ni se crean guests al instalar.

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

## Fuera de alcance

Purga de datos; migración automática de otras instalaciones; auto-updater;
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
