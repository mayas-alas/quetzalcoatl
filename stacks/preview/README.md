# GNX Preview: bundle Linux para infraestructura existente

`https://ui.kernel.gnx` sirve el escritorio publicado por gnxkernel;
`https://notas.gnx` sirve Memos privado; `https://compute.gnx` conserva el PVE real
y su transporte TLS actual. Notas y compute son servicios desplegados, no nuevas
recetas del invitado. Se conserva íntegro el perfil de sesión Debian del desktop.

Este corte es un bundle técnico independiente `gnx-preview-0.1.0.tar.gz`, no una
nueva versión del instalador. [GnxCoreSetup.exe v0.0.41](https://github.com/mayas-alas/quetzalcoatl/releases/tag/v0.0.41)
sigue siendo la vía Windows publicada. No se incorpora Kestra ni se añaden ADO,
selector, plataformas, cuentas de host, bootstrap PVE o recetas de LXC.

## Contrato y preparación

El coordinador proporciona un **LXC nuevo, no privilegiado**, hostname
`gnx-preview`, linux/amd64, con systemd, Podman/Quadlet y skopeo ya disponibles.
Dimensionar inicialmente 2 CPU, 4 GiB RAM y 12 GiB de disco, como el recorrido de
[deployment](../../docs/deployment.md); medir para el uso real. PVE conserva
ownership, almacenamiento, restricciones y lifecycle del guest. No elevarlo para
resolver fallos de Podman. El script de instalación no crea ni adopta guests.

Las unidades `.container` y `.network` son el contrato nativo. Reutilizan las
rutas de configuración y datos y los límites de texto de deployment. El archivo
de distribución completo añade OCI y el desktop; **no se envía a `gnx deploy
<carpeta>`**, cuya admisión no admite payloads OCI/binarios ni realiza builds.
Se aplica dentro del LXC autorizado. systemd/Quadlet gobierna arranque, parada y
reinicio; journal es el diagnóstico. No hay otro supervisor.

[manifest.json](manifest.json) fija versiones, digests linux/amd64, orígenes,
puertos, rutas y volúmenes. La UI usa la imagen oficial Node
`26.13.1-bookworm-slim` y el
[desktop v0.2.0-261127-4d08ae3](https://github.com/mayas-alas/gnxkernel/releases/tag/v0.2.0-261127-4d08ae3),
SHA-256 `4af141d9a8f48cee8cd5389eed5ff5b3651f681f8c11e2039e9bdd67fd03443f`.
No hace `npm install` ni reconstruye WASM o Debian. El preflight del guest exige
esa versión de Node y las APIs JSPI `WebAssembly.Suspending/promising`.

Memos usa la imagen oficial `docker.io/neosmemo/memos:26.11.2`, resuelta por su
manifest amd64 en el Quadlet. Los pins se obtuvieron de los índices oficiales
Docker Hub de ambas versiones, no de un mirror. Las imágenes se archivan con
`skopeo --preserve-digests`; el import al storage nativo conserva la referencia
por digest y `Pull=never` impide descargas al reiniciar. Se comprueba arquitectura
en el empaquetado y existencia local al aplicar. El builder necesita red; el
despliegue consume los dos OCI y el desktop locales.

Memos monta `/var/lib/gnx-app/preview/memos` en `/var/opt/memos`, con UID/GID 10001
del guest. Su [configuración declarativa](https://usememos.com/docs/configuration/declarative-configuration)
fija acceso `PRIVATE` y deshabilita registro abierto antes de servir peticiones;
la URL canónica es `https://notas.gnx`. El primer administrador completa el setup
nativo desde un cliente autorizado. No se distribuyen contraseñas. Restringir
acceso al setup hasta finalizarlo; modo privado no reemplaza ese primer alta ni
revoca enlaces compartidos que un administrador cree posteriormente.

## Empaquetado y aplicación deliberada

La Action [GNX Preview Linux bundle](../../.github/workflows/preview-linux.yml)
valida formato/generación nativa Quadlet y empaqueta los artefactos fijados en PR, push relevante a main o
ejecución manual. No compila el producto, no publica releases ni accede a PVE.
Para reproducir en un builder Linux con Node, curl, skopeo, GNU tar y gzip:

```sh
node stacks/preview/scripts/bundle.mjs
node stacks/preview/scripts/bundle.mjs --package
```

Se requiere checkout comprometido; en PR se usa su commit de cabecera exacto.
El directorio ignorado
`build/gnx-preview-<versión>-<commit>/` conserva el bundle, manifiesto,
`provenance.json` y `SHA256SUMS`. La procedencia identifica commit, epoch del
commit, herramientas, entradas originales y config digests OCI. Tar ordena
nombres y normaliza propietario, modo de formato y timestamps; gzip omite fecha
y nombre. Repetir el mismo commit con las mismas entradas/herramientas produce
el mismo bundle; cambiar herramientas requiere comparar hashes. La procedencia
vincula integridad y origen, no acredita aceptación ni firma de autor.

Tras descargar el artefacto, el coordinador verifica el `SHA256SUMS` exterior
desde una fuente confiable, extrae el bundle, y lo transfiere al guest sin
alterarlo. Desde una consola administrativa del **guest autorizado**:

```sh
bash /ruta/bundle/scripts/install-guest.run /ruta/bundle
```

Es instalación nueva: si hay datos, configuración o unidades, se detiene. Un
fallo parcial conserva lo creado para diagnóstico; no borrar estado para
reintentar a ciegas. Una actualización exige backup de SQLite y adjuntos,
revisión de migraciones y un procedimiento aprobado aparte. Tras la primera
instalación no se ejecuta el instalador para reiniciar: usar systemd. Quadlet
genera el autostart mediante `[Install]`, sin `systemctl enable` de unidades
generadas.

## Ingress en el nodo GNX Linux existente

La vía actual es el nodo GNX que aloja `gnx-core-compute` y su gateway Caddy. Se
reutilizan `/var/lib/gnx-core/stack-gateway`, `stack-dns`, los mounts existentes
y el patrón de recibo `GnX.Core:<digest>`. No se presupone un PVE bare-metal con
esos paths: adaptar el target a otro layout es trabajo administrado separado.

El coordinador copia [target.example.json](target.example.json) **fuera de Git**,
root:root modo 0600, y reemplaza todos los `null` con evidencia del nodo: VMID,
digest del bundle (`SHA256SUMS` exterior), MAC, IP fija del guest, IP del gateway,
next hop e interfaz de la ruta `/32`. La descripción PVE debe estar vinculada a
ese digest desde la creación autorizada; no cambiar una descripción ajena para
forzar adopción. `ca_sha256` es el SHA-256 del DER de la CA **ya verificada** por
el administrador, no un pin elegido del certificado que acaba de presentar una
conexión. Este checkout no fija ni acredita esos valores.

Revisar antes: rutas y puertos libres, firewall del guest limitado al gateway,
gateway montando `stack-gateway`, PVE montando `stack-dns`, CA/CRL persistentes y
resolución confiable de `compute.gnx`. Los puertos 3000/5230 solo son backends
HTTP de la red interna; no exponerlos a clientes ni a Internet. En el **nodo
Linux GNX autorizado**, con el bundle verificado:

```sh
bash /ruta/bundle/scripts/install-ingress.run /ruta/bundle /ruta/privada/preview-target.json
```

No genera una CA: exige la CA persistente y su clave presentes; contrasta el
fingerprint administrado antes de renovar la hoja. `tls.run refresh` conserva
raíz, base de CA, CRL y clave de hoja, e incluye los nuevos SAN. La cadena/nombre
de `compute.gnx` sigue validándose con el mismo root; no se modifica su bloque
Caddy, endpoint PVE ni confianza. Nunca usar `-k` ni agregar una raíz o pin para
superar un fallo. Un target/CA no comprobados quedan pendientes y bloquean aplicar.

`preview-stack.json` registra la instancia por separado del stack privado; un
registro distinto bloquea. El reconciliador contrasta ownership PVE, flag no
privilegiado, MAC/IP, hoja/CA/CRL y estado individual de unidades. Añade solo la
ruta `/32` validada, dos entradas split DNS y un import Caddy fijo. Valida Caddy
con todos los stacks antes de activar el archivo nuevo. El drop-in post-start
del compute repite esa reconciliación al reiniciarlo, sin reemplazar sus hooks.
Un fallo preserva el registro y archivos para inspección; no hay rollback
transaccional ni eliminación automática de recursos.

Pi-hole anuncia los dos nombres a clientes mesh con la IP mesh del gateway;
dnsmasq los anuncia a la red de invitados con la IP interna administrada. La
integración Windows conserva su propia lista de hosts administrados y su
forward/almacén de CA; sus nuevas fuentes incluyen estos nombres, pero modificar
fuentes no actualiza el EXE v0.0.41 ya instalado. El corte Linux no instala ni
requiere Windows/WSL. Los clientes externos usan split DNS y la CA existente.

## Aceptación pendiente desde un cliente confiable

Después de reiniciar unidades UI/Memos, gateway, resolver y compute, comprobar:

1. Resolución de `ui.kernel.gnx`, `notas.gnx`, `compute.gnx` al gateway esperado
   en cada red usada; cadena, SAN, expiración y CRL con la CA ya confiada, sin
   excepciones TLS. HTTPS de compute sigue mostrando el PVE real y permite un
   login nuevo por su autenticación existente.
2. Escritorio carga y `crossOriginIsolated` es `true`; los módulos WASM tienen
   `application/wasm` y arrancan. Sobre la URL Debian `.erofs` que anuncia la UI:
   GET/HEAD, `Range: bytes=0-1023` → 206, `Content-Range`, longitud 1024, ETag
   estable, `If-Range`, `If-None-Match` → 304 y rango fuera de límites → 416.
   Conservar COOP `same-origin`, COEP `require-corp`, CORP `same-origin`, sin
   compresión de la imagen. Arrancar la sesión Debian publicada y su preview.
3. Memos permite setup/admin y login nuevo; en ventana anónima no revela notas
   privadas ni Explore. Guardar nota y adjunto, reiniciar Memos y el guest,
   volver a autenticar y comprobar su persistencia en `/var/opt/memos`. Confirmar
   que no hay registro público abierto.
4. Ambas unidades activas individualmente, ruta `/32`, registro e imports se
   conservan tras reiniciar compute/gateway; no hay pull de imágenes en el corte
   offline ni regresión de los otros stacks, DNS o confianza de compute.

Los checks CI solo prueban formato, integridad y empaquetado. El coordinador
ejecuta y registra ese e2e antes de decidir publicar el release versionado.

Para un instalador propio futuro podrían extraerse el preflight y el contrato de
payload; MSI/Burn, bootstrap WSL/payload, supervisión del servicio y gestión de
hosts/forward/CA pertenecen a la vía Windows. Este bundle Linux no importa esa
capa ni reproduce sus privilegios o transacción de instalación.
