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

El formato está en [config/update-catalog.json](../config/update-catalog.json):
`schema: 1`, `architecture: linux/amd64` y exactamente ocho componentes con
`name` e `image`. Se permiten únicamente los repositorios de esos componentes,
referencias por `@sha256:` y digests hexadecimales minúsculos; no tags, URLs con
credenciales, comandos, rutas de host ni repositorios alternativos. El catálogo
inicial conserva los pins de las fuentes; **no selecciona versiones recientes**.

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
la resolución antes de repetir. No hay rollback automático de imágenes ni de datos:
volver a un digest anterior puede ser incompatible con una migración ya realizada.

En cada inicio solicitado o recuperado del nodo se compara el catálogo local y se
registra el resultado en el Visor de eventos (`GnXCore`). No se consulta Internet,
no se descarga y no se aplica automáticamente. Descubrimiento de novedades remotas,
un canal firmado y ventanas de aplicación automática siguen pendientes.

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
