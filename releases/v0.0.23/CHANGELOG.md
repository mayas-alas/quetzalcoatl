# v0.0.23 — En preparación

**Estado:** en desarrollo; sin entrega ni aceptación funcional.
Entrada enmascarada, actualizador y recreación implementados en fuentes;
compilación, empaquetado y aceptación todavía no establecidos. No están
disponibles en el CLI instalado v0.0.22.

## Añadido en fuentes

- Entrada interactiva de `gnx password` con asteriscos, edición por retroceso y
  cancelación con Esc/Ctrl+C. Conserva la confirmación y no revela la contraseña.
- Validación de la primera entrada antes de pedir confirmación; los rechazos
  indican que no se enviaron cambios al servicio.
- Mensajes que identifican la nueva contraseña de `root@pam`, su persistencia y
  el acceso PVE por Linux PAM. La entrada por stdin conserva dos líneas sin eco.
- Lectura nativa de consola Windows con restauración del modo de entrada,
  manteniendo el pipe autenticado y sin nuevas dependencias de terceros.
- Descubrimiento público en `gnx update --check` sin catálogo explícito:
  incluye pre-releases de producto y verifica la coherencia de sus hashes.
  No descarga los instaladores ni acredita su instalación. Con un catálogo
  explícito se conserva la comprobación local de componentes.

## Documentación

- Definidas las fronteras entre actualización, recreación y operación compartida.
- Documentado el contrato de credenciales de la imagen compute: admite `PASSWORD`
  y `PASSWORD_HASH`, da prioridad al hash no vacío y establece la contraseña de
  root en cada arranque del contenedor.
- Precisados el acceso humano por PVE, el acceso del agente por las interfaces
  autorizadas y la separación respecto a la cuenta Windows de servicio.
- Documentado `gnx password`: nueva contraseña de `root@pam`, confirmación sin
  eco, requisitos de entrada, persistencia del hash y comprobación de login real.

## Actualizador y recreación en fuentes

### Actualización del producto

- `gnx update --apply`: seleccionar la release pública de producto más reciente,
  incluidas pre-releases, verificar el instalador, actualizar GnX mediante los
  mecanismos nativos Windows y aplicar el catálogo aprobado de componentes.
- Actualizar las imágenes conservando volúmenes, datos, identidad y configuración
  declarada. Mantener disponibles los LXC del usuario y los del sistema.
- Resolver el origen desde `GNX_UPDATE_REPOSITORY` y el nombre del MSI desde
  `GNX_UPDATE_MSI_ASSET`, en ámbito máquina y sin almacenar credenciales GitHub.
- Fallar de forma controlada ante un repositorio privado o inaccesible, sin
  instalar, solicitar autenticación ni recurrir a otro origen.
- Worker SCM temporal fuera de `GnXCore`, con ejecutable copiado a estado
  protegido, handoff vinculado al proceso iniciador y salida previa de la CLI.
  Recibo durable y exclusión entre procesos; un resultado desconocido o un
  reinicio requerido bloquea reintentos automáticos. No reinicia Windows.
- Inspección nativa del MSI y mantenimiento por Burn cuando se verifica su
  ownership. Se bloquean instalaciones con guests activos sin fuerza.

### Recreación explícita

- `gnx update --apply --force`: preparar el sistema GnX desde cero, retirando
  el estado persistente e identidades anteriores del entorno autorizado.
- `gnx update --force`: el mismo recorrido destructivo sobre la versión ya
  instalada, sin consultar ni descargar otra release.
- Exigir un inventario de los recursos y datos afectados y autorización para su
  pérdida, incluidos los guests del almacenamiento administrado. No extender
  la operación a Windows, otras WSL o recursos externos al nodo.
- Mantener las verificaciones de integridad y ownership; no introducir prune
  global, reinicios de host ni paradas implícitas de guests.
- Conservar únicamente el scaffold Windows privado de servicio y los archivos de
  transacción; retirar WSL, datos, CA, identidad mesh, credencial PVE y guests del
  almacenamiento propio. Preparar un nodo vacío desde el payload instalado:
  volver a configurar `password` y `enroll`. No se recuperan claves anteriores.
- Bloquear almacenamiento externo/no reconocido o inventarios divergentes.

### Credenciales y operación compartida

- Conservar `gnx password` como flujo explícito para configurar y cambiar la
  credencial PVE persistente. No añadir un gestor paralelo ni una contraseña
  global Windows; revisar la UX del comando sin alterar el contrato del entrypoint.
- Mantener la credencial declarada como fuente de verdad: el entrypoint la
  reaplica al recrear compute, también durante una actualización sin fuerza.
  Es comportamiento de bootstrap, no una corrección pendiente de PVE.
- Un cambio de root realizado solo dentro del contenedor no modifica la
  configuración protegida ni se garantiza tras su recreación. No añadir un
  sincronizador ni recuperar la contraseña privada para el agente. Para un cambio
  persistente se utiliza `gnx password`.

## Arquitectura

- Mantener independientes el sistema base, la configuración y los datos persistentes.
- No incluir en esta entrega inmutabilidad ni rollback de la base WSL/LXC.
