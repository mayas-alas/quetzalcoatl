# Recorridos de usuario

Estos criterios describen el comportamiento esperado del núcleo implementado,
salvo la sección explícita de requisitos pendientes.
No son un registro de ejecuciones ni convierten una comprobación parcial en
aceptación completa. [Alcance](scope.md) delimita el producto;
[Arquitectura](architecture.md) describe sus mecanismos.

## Instalar y acceder

**Como administradora, quiero instalar un nodo separado de mi entorno personal.**

- Usar el EXE como entrada interactiva recomendada o el MSI para administración
  directa. Ambos llevan el mismo núcleo offline; no instalar los dos como pasos.
- Usar un Windows x64 con WSL2 disponible. Informar requisitos o conflictos sin
  adoptar otra cuenta/distribución ni descargar silenciosamente piezas faltantes.
- Crear una identidad interna de servicio y WSL propias, sin inicio humano/RDP.
- No crear aplicaciones ni guests de demostración.
- Mantener/desinstalar con la entrada de Windows correspondiente a la vía elegida.
  La comprobación MSI no se presenta como aceptación del mantenimiento Burn.
- Distinguir el resultado MSI de la preparación Linux y del acceso HTTPS real.

**Como operadora, quiero consultar la app local sin configurar primero la mesh.**

- `https://app.gnx` no espera key Tailscale ni disponibilidad de Proxmox.
- Mostrar información y última fase aplicada sin presentarlas como salud en vivo.
- Gestionar certificados y confianza del host internamente, sin un comando de
  certificados ni credenciales recogidas por el navegador.

## Configurar el nodo

**Como administradora, quiero introducir solo las credenciales necesarias.**

- `enroll` pide la key Tailscale sin eco; `password` hace lo mismo con Proxmox.
- Entregar secretos por el canal local protegido, nunca por argumentos o URLs.
- Un resultado de configuración no acredita acceso remoto ni login humano.
- Compute o mesh pendientes no impiden consultar la app local.

**Como administradora, quiero establecer o cambiar la contraseña de PVE sin
confundirla con mis credenciales Windows.**

- Usar `gnx password` en consola elevada con el nodo iniciado. Establece una nueva
  contraseña de `root@pam`, no solicita la anterior ni cambia la cuenta Windows.
- Introducirla y confirmarla sin eco; documentar el mínimo de 12 bytes y que no
  se revela el texto. Las fuentes 0.0.23 añaden asteriscos, retroceso y cancelación
  con Esc/Ctrl+C en consola interactiva. Mantener la entrada por stdin sin eco
  para automatización y no aceptar secretos como argumentos.
- Persistir el hash protegido y aplicarlo a compute. Distinguir rechazo de entrada,
  fallo de aplicación y resultado desconocido sin presentar estos dos últimos
  como garantía de que la contraseña anterior sigue vigente.
- Usar el mismo comando para un cambio persistente; cambiar root únicamente
  dentro de PVE no actualiza la configuración que se reaplica al recrear compute.
- Comprobar el login real en `https://compute.gnx`, como `root@pam` o `root` en
  Linux PAM, sin imprimir, recuperar ni registrar la contraseña.

**Como operadora, quiero acceder a Proxmox con su autenticación nativa.**

- Abrir `https://compute.gnx` mediante el gateway con TLS backend verificado.
- Conservar el login de Proxmox, sin bypass ni contraseña pública predeterminada.
- No presentar una página de login o un HTTP 200 como prueba de credenciales válidas.
- Para acceso desde otros dispositivos, comprobar DNS, ACL y confianza del cliente.

## Operar y recuperar

**Como operadora autorizada, quiero pausar y reanudar sin perder mis datos.**

- `stop` detiene el target propio; no apaga todas las WSL ni borra identidad.
- La recuperación SCM respeta una parada explícita.
- `start` inicia lo preparado sin reinstalar, actualizar ni exigir reenrolamiento
  si la identidad mesh sigue válida.
- Delegar lifecycle en SCM, systemd/Quadlets y Proxmox, no en una terminal permanente.

**Como operadora, quiero consultar sin cambiar el estado del nodo.**

- `status` no despierta una WSL detenida ni aplica configuración.
- Separar target activo de disponibilidad HTTPS, mesh y compute.
- Informar operación ocupada, fallo o resultado desconocido sin convertirlos en éxito.
- Usar herramientas nativas para diagnóstico ampliado sin mostrar secretos.

## Mantener y retirar

**Como administradora, quiero reinstalar sin perder la identidad del nodo.**

- Actualización y reinstalación conservan cuenta, WSL, CA, credenciales y datos.
- Desinstalar confirma la parada y retira únicamente servicio y acceso Windows propios.
- No continuar silenciosamente ante un conflicto de propiedad o parada fallida.
- La desinstalación no es una purga ni autoriza borrar perfiles, VHDX o respaldos.
- La retirada requiere el servicio Windows disponible y sin una operación en curso;
  `gnx stop` no equivale a detener ese servicio. Un servicio no disponible se
  diagnostica, no se elude borrando archivos o registros.
- La revisión del MSI, su menú o una actualización no acreditan retirada, rollback
  ni reinstalación. Véase [Mantenimiento y desinstalación](docs/uninstall.md).

## Obtener instaladores desde CI

**Como mantenedora, quiero descargar MSI y EXE de una ejecución identificable.**

- Ejecutar manualmente la Action con una release de inputs confiables y SHA-256 conocido.
- Empaquetar el CLI versionado sin sobrescribirlo con una compilación diferente.
- Elegir `cli_mode=source` para compilar una versión nueva en CI, o `versioned`
  para reutilizar el CLI canónico; conservar los hashes y la procedencia técnica.
- Poder empaquetar en el host con los mismos inputs locales y CLI canónico, sin
  presentar esa vía como un producto distinto ni una instalación verificada.
- Conservar los gates de manifiesto, versión e integridad MSI del build canónico.
- Entregar instaladores y hashes como artefacto; publicar una pre-release solo
  con opción explícita y tag coincidente con el commit de ejecución.
- Separar la distribución de tres archivos (setup EXE, MSI y hashes) de la metadata
  técnica de CI. No ofrecer el CLI suelto como un instalador o portable adicional.
- No afirmar instalación, aceptación ni firma de código a partir del empaquetado.

## Despliegue CLI

**Como administradora, quiero enviar una carpeta de Quadlets/configuración para
crear un LXC separado sin convertir el núcleo en otro supervisor.**

- `gnx deploy <carpeta>` transmite archivos admitidos por el pipe administrativo.
- Rust crea un guest nuevo no privilegiado y conserva recibos verificables de propiedad.
- systemd/Quadlets del guest aplican los servicios; no se ejecutan scripts de la
  carpeta en Windows o en el nodo WSL.
- Un fallo conserva el guest parcial; una entrada idéntica ya aplicada no duplica
  LXC ni borra datos. No existe actualización automática de carpetas modificadas.
- Distinguir transferencia y unidades activas de salud HTTP, acceso, login y OIDC.
- Comprobar cada unidad, no solo el resultado agregado de `is-active` de 0.0.16.
  Desde 0.0.17 se consulta cada `ActiveState`; el recibo no certifica salud.
  Reenviar una entrada completada es una consulta,
  no una reparación del guest.

Disponible desde 0.0.16. Subida web, puente nativo y builds no se presentan como implementados.

**Como operadora, quiero utilizar los servicios privados con identidad propia.**

- Desplegar el stack versionado en una red bridge Quadlet, sin `Network=host`.
- Acceder a Pocket ID, RustFS/S3 y NetBird/dashboard por HTTPS del gateway existente.
- Conservar CA, datos y credenciales; no reemplazar una instancia registrada.
- Preparar clientes OIDC distintos para RustFS y NetBird mediante sus APIs nativas.
- Registrar una passkey real del operador y comprobar autenticación/autorización;
  no sustituir ese recorrido por un autenticador simulado o una página de login.
- Comprobar escritura/lectura S3 real y conectividad entre peers por separado.
  El proxy HTTPS no reemplaza el transporte STUN UDP.

## Requisitos pendientes

Estos recorridos corresponden a R-IMG-01, R-LXC-01, R-UPD-01, R-REC-01 y R-VAL-01 en
[alcance](scope.md#requisitos-pendientes-autorizados). Las fuentes 0.0.23 implementan
R-UPD-01 y R-REC-01; su aceptación manual sigue pendiente. R-IMG-01 y R-LXC-01
continúan como evolución pendiente, no como capacidades del instalador.

**Como administradora, quiero instalar una vez con MSI o EXE y actualizar el
producto instalado mediante `gnx update --apply`.**

- Seleccionar una release pública de producto, incluidos candidatos publicados,
  desde el origen administrativo configurado; excluir borradores e inputs técnicos.
- Verificar versión, hashes e identidad del instalador antes de ejecutarlo.
  Un origen privado o inaccesible devuelve fallo controlado sin instalar ni
  recurrir a otro origen o a una sesión de desarrollo autenticada.
- Conservar el mantenimiento nativo MSI/Burn y separar la sustitución del CLI/servicio
  de la aplicación del catálogo. Confirmar cada fase y no repetir una instalación
  cuyo resultado sea desconocido. Sin `--force`, respetar volúmenes, datos,
  identidad, configuración declarada y paradas explícitas. Los LXC del usuario
  y del sistema permanecen disponibles después de la actualización.
- Observar la ventana de compute antes de interrumpirlo; no detener guests por
  iniciativa propia. Comprobar el instalador exacto de CI, no solo un CLI equivalente.

**Como operadora, quiero recrear un entorno propio y compartir su operación con
un agente sin compartir la cuenta Windows de servicio.**

- `gnx update --apply --force` retira el estado anterior del
  sistema GnX y lo prepara desde cero, con identidad nueva y el bootstrap
  configurado. Aprobar antes el inventario de recursos y datos que se perderán,
  incluidos los guests del almacenamiento administrado;
  no extender ese alcance a Windows, otras WSL o recursos ajenos. Reponer solo
  un contenedor conservando sus volúmenes no satisface este recorrido.
- `gnx update --force` permite recrear solo la versión instalada. Se conserva la
  cuenta Windows privada de servicio y la transacción, no el nodo WSL, CA, mesh,
  configuración PVE ni guests anteriores. Reconfigurar `password` y `enroll`.
- Tras la confirmación, retirar la WSL propia mediante `--unregister`, sin VHD de
  respaldo ni parada limpia de PVE como requisito. Verificar ausencia del registro
  y disco anteriores antes del bootstrap; un error no autoriza declarar éxito.
- Usar el lifecycle nativo. Una actualización normal no retira guests; la
  recreación completa retira el almacenamiento del nodo identificado según el
  inventario aprobado. No convierte recursos externos en recursos del producto
  ni omite comprobaciones por recibir un flag de fuerza.
- Configurar la clave PVE acordada mediante `gnx password`, sin crear otro flujo
  de bootstrap ni publicar una variable global Windows con la contraseña.
  Comprobar una autenticación humana real, sin publicar secretos ni recuperar
  contraseñas desde hashes o usar un login abierto como prueba de autenticación nueva.
- Mantener `PASSWORD`/`PASSWORD_HASH` de la configuración protegida como fuente
  de verdad. Recrear compute reaplica esa credencial, también si se conserva su
  almacenamiento durante un update normal. Cambiar root solo dentro del contenedor
  no cambia esa configuración; no se introduce un sincronizador ni recuperación
  de la contraseña privada para el agente.
- Operar la misma instancia por UI humana y CLI/pipe del producto para el agente.
  El mantenimiento nativo PVE/Podman requiere recursos y permisos concretos;
  no se obtiene acceso usando la WSL personal o el login de la cuenta de servicio.
- Separar la aceptación manual de primera preparación, la recreación y el upgrade
  con estado existente. Conservar una ficha local sin secretos para otro modelo;
  documentar estos criterios no autoriza ejecutarlos sin identificar el entorno.

**Como operadora, quiero adoptar nuevas imágenes al iniciar el nodo, incluido el
núcleo, sin perder la capacidad de arrancar offline.**

- Seleccionar canales explícitos por imagen; no convertir un pin en `latest`
  silenciosamente ni confundir digest diferente con versión más reciente.
- Conservar una semilla offline verificada y no reemplazar una imagen promovida
  por la semilla durante la recuperación o aplicación de un paquete.
- Respetar `stop`; `status` no descarga ni inicia servicios. Mantener la coordinación
  en Rust y el lifecycle/descarga en mecanismos nativos.
- Informar qué actualización se comprobó, aplicó, falló o quedó desconocida, sin
  convertir una unidad activa en prueba de salud funcional.

**Como administradora, quiero recrear la base bootc de un LXC y poder volver a
una base anterior si necesito recuperar el guest.**

- Identificar origen, versión/digest, base vigente y base recuperable; comprobar
  compatibilidad real con PVE y el LXC no privilegiado antes de elegir mecanismo.
- Conservar propiedad verificable, identidad, CA, credenciales y datos persistentes
  al recrear o recuperar; no adoptar otro guest ni crear un reemplazo silencioso.
- Distinguir retorno de la base, retorno de imágenes Podman y recuperación de
  datos: bootc no demuestra que una migración de aplicación sea reversible.
- No presentar un rootfs exportado como prueba de rollback bootc dentro del LXC.
  Comprobar recuperación y límites antes de declarar aceptación, con autorización
  específica para las operaciones que muten la instalación.
