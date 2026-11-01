# Actualización de componentes

Las fuentes 0.0.20 añaden una operación administrativa al pipe existente. Requiere
CLI y servicio de esa versión instalados; ejecutar un CLI recién compilado contra
un servicio anterior no actualiza el producto ni incorpora sus operaciones.

## Comprobar y aprobar

En PowerShell elevado, usando la instalación:

```powershell
$gnx = "$env:ProgramFiles\GnX Core\gnx.exe"
& $gnx update --check
& $gnx update --check C:\Administracion\catalogo.json
& $gnx update --apply C:\Administracion\catalogo.json
```

Sin archivo se usa el último catálogo aplicado explícitamente; si aún no existe,
se usa el incorporado en el CLI. `--check` con archivo solo previsualiza: no guarda
la aprobación. Después de verificar las descargas, `--apply` conserva el catálogo
en estado Windows protegido. Un cambio del CLI no reemplaza esa aprobación.

Las fuentes 0.0.23 añaden, antes de esa comparación, descubrimiento público de
releases cuando `--check` no recibe un catálogo. El origen se toma de
`GNX_UPDATE_REPOSITORY` y el nombre MSI de `GNX_UPDATE_MSI_ASSET`, de ámbito
máquina; sus valores predeterminados son `mayas-alas/quetzalcoatl` y `GnxCore.msi`.
Se incluyen pre-releases con tag numérico de producto, no borradores ni releases
de inputs. Se verifica `SHA256SUMS.txt` contra su digest publicado y la coherencia
con los digests de ambos instaladores; no se descargan los instaladores.
Un origen inaccesible o metadatos incoherentes devuelven error, sin autenticación
ni cambio de origen. Para comprobar solo componentes sin red, aportar un catálogo
explícito.

## Actualización del producto en 0.0.23

Sin catálogo, `gnx update --apply` selecciona la release pública numéricamente
mayor, incluidos candidatos publicados, y rechaza downgrades. Si la versión ya
coincide, aplica explícitamente el catálogo incorporado. Para una versión mayor
descarga MSI y hashes; cuando la instalación pertenece a Burn, conserva esa vía
mediante el EXE de la misma release. Una identidad/versionado MSI o Burn ambiguos
bloquean la operación, sin adoptar recursos por nombre.

Un worker SCM temporal verifica el inventario y bloquea la instalación normal
con guests activos. La CLI sale antes de que el worker detenga el servicio e
invoque el instalador nativo; después recupera `GnXCore` y aplica el catálogo del
payload instalado. La entrega al worker no confirma su finalización. Consultar
`gnx update --check`, el recibo Windows protegido y el Visor de eventos. Un fallo
tras iniciar mantenimiento, timeout o reinicio requerido bloquea reintentos
automáticos: resolver el estado nativo antes de continuar. No hay reboot implícito
ni rollback automático de migraciones.

El CLI de 0.0.22 no descarga MSI: la primera instalación de 0.0.23 requiere ejecutar
su MSI o EXE publicado respetando la vía de mantenimiento existente.

0.0.24 normaliza el orden del conjunto `content` que devuelve PVE y precisa qué
parte del inventario diverge. El CLI instalado 0.0.23 conserva su comparación
anterior y puede bloquear antes de lanzar ese MSI. Si sucede, usar el instalador
nativo de la candidata en una ventana con guests detenidos; no editar recibos ni
omitir la comprobación para forzar el salto.

## Recreación explícita en 0.0.23

`gnx update --force` recrea la versión instalada, sin descubrimiento remoto.
`gnx update --apply --force` selecciona primero la release pública y después
recrea. Ambas formas requieren consola interactiva, inventario verificado y
escribir `RECREAR`; cancelar no autoriza instalación ni borrado.

Se retiran la WSL propia, sus datos, CA, identidad mesh, credencial/configuración
PVE y todos los guests del almacenamiento administrado, incluidos los del usuario.
No hay recuperación automática. Se conserva la cuenta Windows privada de servicio
y la transacción, no el nodo anterior. El payload instalado prepara un nodo vacío;
volver a configurar `gnx password`, `gnx enroll` y los despliegues deseados.
Almacenamiento externo, estado no reconocido o un inventario que cambió bloquean
la recreación. Windows, otras WSL y recursos externos no se purgan.

El formato está en [config/update-catalog.json](../config/update-catalog.json):
`schema: 1`, `architecture: linux/amd64` y exactamente ocho componentes con
`name` e `image`. Se permiten únicamente los repositorios de esos componentes,
referencias por `@sha256:` y digests hexadecimales minúsculos; no tags, URLs con
credenciales, comandos, rutas de host ni repositorios alternativos. El catálogo
incorporado en 0.0.22 refresca los pins a las últimas estables del momento de la
release (verificadas contra los registros: netbird-server 0.82.0, netbird-dashboard
v2.96.0, mesh v1.104.4, dns 2026.10.1, gateway Caddy 2.11.7; pocket-id, rustfs y
compute sin cambio). No descubre ni adopta automáticamente `latest`.

Antes de aprobar otro digest, el administrador revisa la release oficial, plataforma,
compatibilidad entre servidor/dashboard y migraciones, registra la procedencia y
dispone de recuperación de datos. La validación sintáctica del CLI no acredita
esa revisión. Un hash da integridad; el archivo local se acepta por la autoridad
administrativa autenticada del pipe, no como un manifiesto firmado por el fabricante.

## Aplicación y conservación

Rust comprueba la identidad Windows/WSL y contrasta el stack con sus recibos Windows,
VMID, descripción PVE y MAC. Solo actúa sobre sus cuatro servicios del núcleo y las
cuatro unidades del stack privado registrado. No crea otro LXC ni adopta recursos
por coincidir en nombre. Las demás clases de despliegue no se actualizan aquí.

1. Observa imagen/configuración/unidad y bloquea pendientes desconocidos o divergencias.
2. Descarga las candidatas activas por digest y comprueba plataforma antes de cambiar
   unidades. Un fallo en esa fase no reinicia servicios, aunque puede dejar imágenes
   descargadas en caché.
3. Guarda el catálogo aprobado y aplica, en orden, aplicaciones, DNS, gateway, mesh
   y compute. Solo cambia `Image`/`Pull` en los Quadlets existentes y conserva el
   resto de configuración. Systemd sigue siendo dueño de su lifecycle.
4. Conserva copia de cada Quadlet anterior y recibos bajo `updates/` en el estado
   Linux propio. Confirma ID ejecutado, unidad activa y healthcheck si lo hay.
   No presenta esos controles como login, conectividad o aceptación funcional.

Las unidades detenidas permanecen detenidas y se informan sin actualizar. Compute
con un cambio pendiente se bloquea si hay guests ejecutándose: planificar una
ventana y detenerlos explícitamente mediante PVE antes de aplicar. No se detienen
guests ni se reinicia el host por iniciativa del actualizador.

Un fallo de aplicación puede dejar componentes anteriores ya actualizados. Se
detiene el resto, devuelve fallo y conserva `pending.json` por componente. Un
timeout no demuestra salida; `--check`, estado systemd, Podman y journal orientan
la resolución antes de repetir. No hay retorno automático a la imagen anterior.
Por arquitectura, selección de imagen y estado persistente son capas separadas:
los datos se conservan al cambiar de imagen y la reversibilidad de una migración
se evalúa por aplicación. Volver a un digest anterior requiere esa compatibilidad.

En cada inicio solicitado o recuperado del nodo se compara el catálogo local y se
registra el resultado en el Visor de eventos (`GnXCore`). No se consulta Internet,
no se descarga y no se aplica automáticamente. Descubrimiento de novedades remotas,
un canal firmado y ventanas de aplicación automática siguen pendientes.

## Etiqueta PVE de guests

Los guests creados por el producto llevan desde 0.0.22 el tag PVE `gnx` desde su
creación. Al aplicar una imagen al stack privado, `update --apply` añade el tag
solo después de validar ownership del guest (description `GnX.Core:<digest>`,
sin privilegios y MAC registrada),
conservando los demás tags existentes y comprobando el digest de configuración
PVE para no sobrescribir cambios concurrentes. `--check` no escribe tags.
Un guest que no pasa esa validación —incluidos los LXC creados por el operador
fuera del producto— no se modifica; retirar un tag aplicado externamente es
una operación manual del administrador en PVE.

## Consola RustFS

RustFS 1.0.4 incorpora Console v0.1.37. El frontend v0.1.38 corrige el bucle de
montaje/refresco para usuarios no administradores ([issue #271](https://github.com/rustfs/console/issues/271),
[fix #272](https://github.com/rustfs/console/pull/272)). No requiere ampliar `readwrite`
a `consoleAdmin`, cambiar Pocket ID ni reconstruir el servidor RustFS.

La entrega incorpora el ZIP oficial definido en [config/console.json](../config/console.json),
verificado por SHA-256. La preparación extrae sus archivos en
`/var/lib/gnx-core/web/rustfs-console-v0.1.38`; Caddy los sirve en
`https://rustfs.gnx/rustfs/console/`. Conserva las rutas nativas de API, S3, callback
y metadatos. `X-GnX-RustFS-Console: v0.1.38` identifica las respuestas estáticas;
`/rustfs/console/version` sigue mostrando la versión del servidor.

La consola se actualiza con la entrega de GnX, no con los digests del catálogo.
`update --apply` puede preparar sus archivos offline pendientes y reiniciar solo
el gateway si la configuración coincide con la entrega. La comprobación humana del
dashboard sigue siendo independiente. No modificar chunks minificados ni aceptar
una carpeta existente sin contrastar todos sus archivos con el ZIP oficial.

Cuando una imagen oficial RustFS compatible incluya el arreglo, se podrá retirar
esta separación mediante otro cambio versionado y verificado. No basta con cambiar
el pin del servidor y asumir qué frontend incorpora.
