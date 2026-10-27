# Compilación y empaquetado

## Herramientas y salidas

- Windows x64, Rust/Cargo compatible con `Cargo.toml` y `Cargo.lock`.
- .NET SDK 8 para WiX 5.0.2 y Burn; no es requisito runtime del producto.
- Inputs offline preparados y confiables, sin secretos ni identidades de instancias.
- Espacio libre suficiente para payload, CAB, validación MSI y bundle EXE. CI exige
  al menos 12 GiB libres antes de descargar; paquetes mayores pueden necesitar más.

El build canónico es `scripts/build.ps1`. Las salidas quedan en
`build/releases/v<versión>/`: `GnxCore.msi`, `GnxCoreSetup.exe` y `gnx.exe`.
El directorio `releases/v<versión>/` conserva CLI y changelog versionados, no MSI/EXE.

## Responsabilidades y distribución

| Pieza | Contrato |
| --- | --- |
| `GnxCoreSetup.exe` | Entrada interactiva recomendada. Burn ejecuta el MSI incorporado y gestiona el mantenimiento del bundle; no contiene otro provisioning. |
| `GnxCore.msi` | Instalación administrativa directa. Es el único paquete dueño de archivos, servicio y transacción Windows. |
| Servicio `GnXCore` | Preparación y operación de la identidad/WSL y aplicación Linux, fuera de la transacción de copia del MSI. |

Ambas descargas incluyen el mismo payload offline. Un tamaño similar no implica
dos núcleos distintos ni obliga a instalar ambas. No se añaden descargas,
prerrequisitos ni lógica en el EXE para justificar artificialmente su existencia.
El usuario mantiene/desinstala mediante la entrada de Windows de la vía elegida.
La aceptación MSI no acredita el ciclo completo de mantenimiento Burn.

La release de usuario contiene exactamente tres assets: setup EXE, MSI y
`SHA256SUMS.txt` con dos entradas. `gnx.exe` no es portable ni se publica suelto
como una tercera opción. Digest base, manifiesto y CLI permanecen en el artefacto
técnico; los inputs offline pertenecen a su release de CI, no a la de instalación.
GitHub añade enlaces «Source code» automáticamente: son fuentes, no instaladores.

## Compilar el CLI

```powershell
./scripts/build.ps1
```

El script compila con `cargo build --release --locked`, comprueba `--version`,
exige el changelog de la versión y conserva el CLI en `releases/v<versión>/gnx.exe`.
Rechaza un binario distinto si esa versión ya existe. Un cambio del binario exige
nueva versión; no eliminar ni sustituir el existente para eludir el gate.
La versión procede del workspace `Cargo.toml`.

## Inputs offline

El checkout no contiene los archivos grandes de empaquetado:

```text
build/payload/rootfs.tar
build/payload/images/mesh.oci.tar
build/payload/images/dns.oci.tar
build/payload/images/gateway.oci.tar
build/payload/images/compute.oci.tar
build/releases/v<versión>/base-image.digest
```

`rootfs.tar` es el filesystem preparado para WSL2, con systemd, Podman y dependencias.
Las imágenes corresponden a `config/images.json`. `base-image.digest` identifica
la imagen base sellada que produjo ese filesystem: `sha256:` y 64 caracteres hex
minúsculos, sin salto de línea. No es el hash del archivo `rootfs.tar`.

La preparación de esos inputs es responsabilidad del mantenedor; no existe una
descarga pública implícita. Se distribuyen como un asset de una release de inputs
del mismo repositorio, independiente de los instaladores. Deben proceder de bases
limpias, no de una WSL enrolada. Antes de distribuir, comprobar origen, licencias,
arquitectura e IDs OCI frente a la configuración del checkout. Los hashes prueban
integridad, no que un filesystem sea seguro o esté libre de secretos.

`-SealBase` es una operación de mantenimiento existente: exige
`build/tools/crane/crane.exe`, usa `config/bootc.json` y prepara la base sellada.
No obtiene las otras cuatro imágenes ni es un bootstrap completo de CI.

Con los inputs preparados:

```powershell
# Compila y empaqueta; exige coincidencia con el CLI versionado si ya existe.
./scripts/build.ps1 -Package

# Solo empaqueta el CLI ya versionado, sin recompilarlo.
./scripts/build.ps1 -Package -UseVersionedBinary
```

Ambas rutas verifican versión, hashes del CLI, manifiesto del payload y versión/hash
del CLI incorporado al MSI antes de construir el EXE. WiX mantiene sus validaciones.
La segunda ruta evita confundir una recompilación con otro toolchain con el binario
canónico; no demuestra que las fuentes actuales se hayan recompilado.

## GitHub Actions

[Windows installers](../.github/workflows/installers.yml) se ejecuta manualmente
con `workflow_dispatch`, en `windows-2022`. No se activa por pull requests ni instala
el producto. El job de empaquetado usa solo lectura. El job opcional de publicación
requiere escritura y publica únicamente una pre-release tras completar los gates.

### Preparar el ZIP de inputs

El ZIP debe contener exactamente estos ocho archivos, sin directorio envolvente
ni entradas de directorio adicionales:

```text
rootfs.tar
images/mesh.oci.tar
images/dns.oci.tar
images/gateway.oci.tar
images/compute.oci.tar
base-image.digest
config/images.json
config/bootc.json
```

Las dos configuraciones son copias exactas de las del checkout, para detectar
incompatibilidades del paquete. Los scripts y configuración runtime se toman del
repositorio, no del ZIP. No incluir el payload de una instalación ni su estado.

Ejemplo en un equipo de preparación, con inputs ya disponibles, desde la raíz
del repositorio. Obtener la versión del workspace, no de una release anterior:

```powershell
$metadata = cargo metadata --no-deps --locked --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE) { throw 'No se obtuvo la versión del workspace' }
$version = ($metadata.packages | Where-Object name -eq 'gnx').version
$archive = Join-Path $PWD 'build/gnx-offline-inputs.zip'
tar.exe -a -cf $archive -C build/payload rootfs.tar images/mesh.oci.tar images/dns.oci.tar images/gateway.oci.tar images/compute.oci.tar -C "../releases/v$version" base-image.digest -C ../../.. config/images.json config/bootc.json
if ($LASTEXITCODE) { throw 'No se creó el ZIP de inputs' }
Get-FileHash -LiteralPath $archive -Algorithm SHA256
```

Adjuntar ese ZIP como `gnx-offline-inputs.zip` a una release de inputs del mismo
repositorio. La Action descarga mediante `gh` y su token automático, también en
repositorios privados. No introducir tokens en parámetros ni hacer público el
repositorio para habilitar descargas. Conservar el SHA-256 por un canal confiable.

Inputs publicados con el tag
[offline-inputs-v0.0.14](https://github.com/mayas-alas/quetzalcoatl-next/releases/tag/offline-inputs-v0.0.14),
asset `gnx-offline-inputs.zip`, SHA-256
`9ff2b812c5227c7163fbb6f7e92b8e5f9466990914d5c8bed5523f517c84e3a4`.
Ese tag identifica los inputs, no la versión obligatoria del CLI: también son
los inputs usados por 0.0.16. Solo se pueden reutilizar mientras coincidan
configuración, arquitectura, IDs OCI y contrato de integridad del checkout.

### Ejecutar y descargar

1. Asegurar que el ref elegido contiene CLI y changelog de la versión del workspace,
   la configuración compatible y todos los scripts runtime.
2. En GitHub, abrir **Actions → Windows installers → Run workflow**.
3. Indicar `payload_release` (tag de la release de inputs) y `payload_sha256`
   (hash del ZIP completo). La Action descarga, verifica y extrae solo los archivos permitidos.
4. Se ejecuta `build.ps1 -Package -UseVersionedBinary`, sin reemplazar el CLI canónico.
5. En **Artifacts**, elegir según la finalidad:
   - `gnx-core-v<versión>-windows-x64`: MSI, setup EXE y `SHA256SUMS.txt`.
   - `gnx-core-v<versión>-build-metadata`: CLI canónico, digest base, manifiesto,
     changelog y `BUILD-SHA256SUMS.txt`. No es una instalación portable.
   Retención: siete días. Los hashes técnicos y los de distribución están separados.
6. Opcionalmente, activar `publish_release` y ejecutar desde el tag `v<versión>`
   previamente creado sobre el commit elegido. Tras el build, un segundo job
   verifica hashes y coincidencia tag/commit; crea una release borrador con sus
   tres assets de usuario y solo entonces la publica como pre-release. No publica
   archivos técnicos ni sustituye releases existentes.

No se suben resultados parciales si falla el empaquetado. Los instaladores no se
firman con Authenticode por este workflow. Los hashes no sustituyen una firma de
código. No se afirma equivalencia byte a byte de MSI/EXE entre ejecuciones.
Un mismo CLI y payload no demuestran que se haya instalado el MSI/EXE exacto
publicado: la comprobación local y la del instalador distribuido deben identificarse
por separado. `source_commit` registra el HEAD al empaquetar, no el diff sin commit;
contrastar también el árbol de trabajo antes de atribuir un compilado a unas fuentes.

## Comprobación y limpieza

Empaquetado correcto no acredita instalación, URLs disponibles, login o recuperación.
Esas comprobaciones requieren un host autorizado y se informan por separado.
No instalar en el runner para fingir aceptación ni reutilizar un host personal como CI.
La retirada tiene requisitos propios: [Mantenimiento y desinstalación](uninstall.md).
No iniciarla como parte implícita de una auditoría de empaquetado.

El build elimina únicamente copias MSI/EXE intermedias idénticas a la salida y
CAB cuya incorporación se verifica en el MSI. Para setups antiguos,
`scripts/clean-release-exes.ps1` muestra el plan; `-Apply` conserva el setup más
reciente verificado, todos los MSI y CLI versionados. No limpia datos ni cachés
de Windows Installer.
