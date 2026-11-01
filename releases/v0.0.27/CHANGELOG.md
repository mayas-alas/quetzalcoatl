# v0.0.27 — Candidata

**Estado:** correcciones de bootstrap y retirada; aceptación en host pendiente.

- `config/wsl.conf` es la fuente de `/etc/wsl.conf`: se aplica por stdin, sin
  necesitar una unidad Windows montada para configurar WSL. Explicita automontaje
  sobre `/var/mnt/` y evita añadir el PATH Windows al entorno Linux. El archivo
  `config/windows-wslconfig` mantiene la configuración Windows del perfil dedicado.
- El bootstrap no depende de DrvFS para leer el payload. `tar` nativo entrega por
  stdin únicamente los archivos del manifiesto verificado, salvo el rootfs ya
  importado. Linux comprueba el manifiesto y los hashes antes de ejecutar el
  instalador existente por systemd. No hay un nuevo supervisor, dependencia ni
  archivo tar temporal en Windows. El staging Linux se retira solo tras éxito.
- Los comandos Linux usan `--cd /`; no necesitan traducir el directorio Windows
  del perfil. Los fallos de bootstrap conservan etapa, código y detalle acotado.
- El SID/GUID propio se registra tras la importación, antes de aplicar Linux,
  para reconocer una preparación incompleta sin confundirla con otro nodo.
- Una nueva solicitud `--force` puede inventariar el nodo parcial propio tras una
  recreación interrumpida. Muestra su GUID actual y exige `RECREAR` para retirarlo
  mediante `--unregister`; no reutiliza la autorización ni aparenta conservar datos.
  Si no existe nodo, se mantiene la confirmación `PREPARAR` sin otro borrado.
  Si el registro propio sigue presente pero su VHD ya falta, la retirada verifica
  igualmente SID/GUID/BasePath y permite el desregistro nativo sin adoptar otra ruta.
- La desinstalación nativa reconoce propiedad sin exigir bootstrap completo,
  detiene solo unidades propias existentes y no exige un certificado que todavía
  no se publicó. No borra cuenta, WSL/VHD ni datos. Una operación interrumpida exige
  comprobar el cliente MSI SYSTEM, ausencia de worker/procesos previos y bootstrap
  Linux inactivo antes de retirar; resultados desconocidos siguen bloqueados.
- El rollback de una instalación incompleta sin intención declarada no inventa
  un arranque ni exige un archivo de intención que todavía no existe.

La síntesis de políticas y fronteras está en [lifecycle](../../docs/lifecycle.md).
La compilación y el empaquetado no acreditan instalación, recreación, login ni el
ciclo de desinstalación/rollback/reinstalación en un host.
