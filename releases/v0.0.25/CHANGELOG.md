# v0.0.25 — Candidata

**Estado:** corrección del recorrido destructivo; aceptación real pendiente.

## Recreación del nodo

- Tras inventario y `RECREAR`, `update --force` retira exclusivamente la WSL
  identificada por SID, GUID y BasePath mediante `wsl --unregister`. No usa la
  retirada conservadora de desinstalación ni exige una parada ordenada de PVE.
- No se conserva el VHD, sus volúmenes ni sus guests. Se comprueba que el registro
  anterior y el VHD desaparecieron antes de importar la base del payload instalado.
  Un error nativo se informa; no se declara éxito ni se restaura el nodo anterior.
- Con WSL/PVE detenida, la lista de guests puede ser histórica o no observable.
  Se señala expresamente; la confirmación abarca TODO el contenido de ese VHD,
  también guests no observables. No se adopta otra distribución por su nombre.
- Después de confirmar se revalida la identidad Windows/WSL y los recibos propios,
  no la salud ni los estados cambiantes de los guests que se van a eliminar.
- Una retirada previa interrumpida sin instalación MSI puede sustituirse por una
  nueva solicitud destructiva: exige la misma identidad, ausencia de worker y de
  procesos anteriores, verificación de archivos y otra confirmación. No reanuda
  automáticamente ni permite repetir resultados desconocidos del instalador.
- El subproceso registra su error específico en el Visor de eventos y el worker
  conserva el código de salida. La recreación no abre una sesión para mantener
  vivo el nodo que se va a retirar; la actualización conservadora sí la conserva.
- Cancelar o fallar una nueva solicitud antes de la operación nativa no libera
  el bloqueo de una recreación anterior sin resolver. El recibo anterior permanece
  durante la preparación y el reemplazo conserva esa barrera hasta completar.

Se conservan únicamente el scaffold Windows privado de servicio y los archivos
de transacción. Windows, otras WSL y almacenamiento externo no se purgan. El nodo
nuevo requiere `password`, `enroll` y los despliegues deseados de nuevo.
