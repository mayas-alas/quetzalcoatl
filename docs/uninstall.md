# Mantenimiento y desinstalación

## Entrada de Windows

Usa **Configuración → Aplicaciones → Aplicaciones instaladas → GnX Core** y la
entrada correspondiente a la vía con la que instalaste:

- **MSI directo:** Windows Installer gestiona modificación y desinstalación.
- **EXE Burn:** el bundle gestiona su mantenimiento y delega la retirada del
  producto en su MSI. El MSI encadenado no se ofrece como otra app independiente.

No instales la otra vía para completar una instalación ni borres registros,
archivos del producto o cachés de Windows Installer para forzar la retirada.
Que aparezca «Desinstalar» no demuestra que la operación haya terminado bien.

## Requisitos de la retirada

La acción del MSI usa el servicio Rust y su pipe administrativo existentes. En
0.0.16 necesita `GnXCore` en ejecución, con identidad verificable y sin una
operación en curso. No arranca automáticamente un servicio Windows detenido ni
incorpora una vía alternativa que omita esas comprobaciones.

Desde 0.0.27 también se reconoce un nodo parcial mediante el recibo de identidad
o una recreación interrumpida propia, contrastando SID, GUID y BasePath. No se exige
que existan todas las unidades o un certificado aún no publicado. Se conservan los
datos; una operación MSI desconocida, worker/proceso previo o bootstrap Linux activo
siguen bloqueando la retirada. Para una recreación interrumpida, la excepción de
mantenimiento exige el cliente SYSTEM real del instalador, observado por el pipe.

`gnx stop` detiene el nodo, pero deja el servicio Windows disponible para recibir
operaciones. Detener `GnXCore` desde el administrador de servicios es distinto y
puede impedir la retirada hasta recuperar ese servicio. Si faltan archivos,
identidad o acceso al pipe, diagnosticar el fallo; no sustituirlo por borrado manual.

## Qué intenta retirar

1. La acción interna solicita parar `gnx-core.target` y comprueba que el target,
   los cuatro servicios del núcleo y el mantenimiento de CRL no sigan activos.
2. Verifica los recibos de los forwards locales y la línea marcada de `hosts`.
   Un conflicto de propiedad bloquea la retirada, en lugar de modificar recursos ajenos.
3. Retira esos forwards y esa línea de `hosts`, y elimina del almacén raíz Windows
   el certificado público exacto registrado por el producto. La CA persistente y
   su clave no se borran de WSL. No hay un paso manual de certificados para el usuario.
4. Invalida `applied-version`. Después, Windows Installer detiene/elimina el
   servicio y retira los archivos instalados, incluidos CLI y payload.

La retirada explícita y una actualización son caminos distintos: el MSI no ejecuta
la acción de retirada del nodo cuando Windows Installer está reemplazando la
versión anterior durante una actualización mayor.

## Qué conserva

| Recurso | Después de una retirada normal |
| --- | --- |
| Cuenta dedicada, perfil y registro/VHDX de WSL | Se conservan. |
| Estado protegido Windows, credencial DPAPI y recibos de despliegue | Se conservan; `applied-version` se invalida. |
| CA, claves y credenciales Linux | Se conservan en el estado propio. |
| Configuración PVE, discos LXC y datos de aplicaciones | Se conservan en los directorios persistentes del nodo. |
| Copias del instalador descargadas o respaldos externos | La retirada del producto no los elimina. |

No se ejecutan `pct destroy`, desregistro WSL, borrado de cuentas ni una purga de
datos. Retirar Windows Installer no equivale a borrar el nodo persistente. Tampoco
borra identidades o políticas que existan en servicios externos.

## Fallos y límites de recuperación

Un error de la acción interna bloquea la retirada. La parada del nodo y los
cambios de acceso Windows no son una única transacción atómica: puede existir
estado parcial si falla una fase. El rollback del MSI intenta aplicar la intención
guardada mediante `start` o `stop`; no debe interpretarse como garantía de que
todas las fases, rutas de mantenimiento Burn o datos de aplicaciones se recuperaron.

Consultar el log de Windows Installer, eventos `GnXCore`, journal y tareas PVE antes
de repetir una operación desconocida. No borrar recibos para evitar un conflicto.
La [síntesis de lifecycle](lifecycle.md) distingue estas fronteras de `--force`:
desinstalar no ejecuta `--unregister` ni elimina el VHD del nodo.

La inspección del MSI o una actualización satisfactoria no acreditan el ciclo de
desinstalación, rollback y reinstalación de esa versión. Ese ciclo requiere una
comprobación autorizada separada, con conservación de identidad y datos contrastada.
