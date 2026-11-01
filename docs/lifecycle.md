# Síntesis de lifecycle — objetivo 0.0.27

**Objetivo:** un único bootstrap y controles de propiedad compartidos, con políticas
distintas para actualizar, desinstalar y recrear. La referencia 0.0.20 orienta la
separación nativa; no se restauran sus fuentes ni se omiten correcciones posteriores.

| Operación | Política sobre el nodo propio |
| --- | --- |
| Actualizar | Conservar identidad, configuración, volúmenes y guests. |
| Desinstalar | Retirar publicación y producto Windows por MSI/Burn; conservar cuenta, WSL/VHD y datos según [el contrato de retirada](uninstall.md). |
| `update --force` | Inventariar y confirmar; `wsl --unregister` de SID/GUID/BasePath verificados; comprobar ausencia del GUID/VHD anterior; ejecutar bootstrap desde cero. |

El bootstrap crea **otro VHD**: su existencia posterior no demuestra que se
conservó el anterior. Si falla, ese estado parcial se identifica y se conserva
hasta otra autorización; ni reinstalar ni cambiar un recibo equivalen a terminar.

## Camino a la síntesis

1. **Taxonomía:** producto Windows, identidad Windows, nodo WSL y guests son
   recursos diferentes. `stop`, retirada conservadora y borrado no son sinónimos.
2. **Configuración:** `config/windows-wslconfig` gobierna `.wslconfig` del perfil
   dedicado; `config/wsl.conf` gobierna `/etc/wsl.conf`. Nunca la WSL personal.
   Aplicar este último por stdin evita necesitar DrvFS para configurar DrvFS.
   El bootstrap traspasa los archivos verificados por
   stdin con `tar` nativo, comprueba hashes en Linux y sigue con systemd; sin otro
   servicio de transporte ni cambio de protecciones Windows.
3. **Merge semántico:** centralizar SID/GUID/BasePath en `identity`, publicación
   en `access` y preparación en `bootstrap`; reutilizar esas fronteras sin mezclar
   las políticas destructiva y conservadora ni crear otros supervisores.
4. **Traspaso y gobernanza:** SCM, lease y recibo deben conservar actor, recurso,
   fase y resultado nativo. Una interrupción bloquea repetición desconocida, pero
   no debe confundirse con instalación fallida o justificar purgas globales.
5. **Retirada incompleta:** permitir al MSI comprobar y retirar publicación de
   un nodo parcial propio sin exigir que ya esté preparado ni borrar sus datos;
   mantener bloqueos por ownership, procesos activos o resultado desconocido.
6. **Entrega comprobable:** formato/análisis no son aceptación. Conservar el CLI
   canónico de Actions y sus hashes; contrastar instalación, `--force`, bootstrap
   y desinstalación en el host por separado, sin declarar éxito por SCM `Running`.

Fuera de alcance: otras WSL/cuentas, borrado de almacenamiento externo, restaurar
datos eliminados, nuevos flags de build, dependencias o refactors generales.
Un fallo de transporte DrvFS no se declara corregido por escribir configuración:
hay que comprobar acceso real al payload antes de continuar.
