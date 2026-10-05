# GnX Node — arquitectura vigente

## Decisión

GnX Node instala un servicio Windows Rust, una identidad dedicada `gnxnodesvc`
y su distro WSL2 `gnx-node`. Podman/systemd ejecutan cuatro capacidades:

| Capacidad | Implementación | Contrato |
| --- | --- | --- |
| `mesh` | Tailscale | Red privada, `/dev/net/tun` |
| `dns` | Pi-hole | Split DNS privado `gnx` |
| `gateway` | Caddy | HTTPS para `app.gnx`, `compute.gnx` y `rustfs.gnx` |
| `compute` | Imagen sellada Proxmox | KVM y HTTPS interno `8006` |

Desde v0.1.2, **Setup es una CLI Rust mínima**. v0.1.4 la publica como `gnx` en el PATH del host, permite rotar de forma segura la contraseña de Proxmox y conserva el flujo de actualización. No hay WebView2, assets locales
de Setup ni puente de credenciales en el navegador. `https://app.gnx` conserva
exclusivamente la vista cotidiana de estado y acceso a compute.

La sustitución afecta la presentación del alta, no el servicio, protocolo,
identidad dedicada, distro, imágenes ni límites de confianza.

## Instalación y uso

1. Burn eleva e instala el MSI. El MSI copia archivos y registra/inicia
   `GnXHostAgent`; no ejecuta provisioning Linux dentro de su transacción.
2. Al terminar, el usuario pulsa **Launch** para abrir `gnx --pause` elevado.
   Burn usa `ApprovedExeForElevation` y la ruta escrita por el MSI en HKLM.
   Las instalaciones silenciosas no abren prompts. Con MSI directo se ejecuta
   la consola instalada manualmente como administrador.
3. En un nodo nuevo, la CLI solicita contraseña Proxmox y confirmación con eco
   desactivado; envía `Provision`. Si alcanza `MESH_PENDING`, continúa en la misma
   ejecución solicitando la clave Tailscale `tskey-auth-*` y envía `JoinMesh`.
4. Un nodo `NODE_READY` no vuelve a pedir credenciales. Un nodo ya inscrito con
   fallo de verificación conserva sus credenciales; se reconcilia con `Retry`,
   no se interpreta el fallo como permiso para volver a inscribirlo.
5. El host Windows local recibe automáticamente la confianza de la CA pública
   del nodo y acceso HTTPS loopback a `app.gnx`/`compute.gnx` antes de declarar
   `NODE_READY`. El administrador completa Split DNS y confianza de la CA pública
   solo en clientes tailnet externos cuando sean necesarios. La interfaz cotidiana
   queda en `https://app.gnx`; Proxmox en `https://compute.gnx`.

```powershell
# En PowerShell elevado, después de instalar:
& "${env:ProgramFiles(x86)}\GnX Node\gnx.exe"
& "${env:ProgramFiles(x86)}\GnX Node\gnx.exe" status
& "${env:ProgramFiles(x86)}\GnX Node\gnx.exe" set password
& "${env:ProgramFiles(x86)}\GnX Node\gnx.exe" retry
```

El MSI agrega el directorio instalado al PATH de sistema. Una terminal nueva del usuario host puede invocar `gnx`; la identidad dedicada sigue sin login interactivo y no ejecuta la CLI.

Comandos: alta guiada sin argumentos o con `setup`; `status`, `set password`,
`join`, `retry`, `cancel`, `help`, `version`. `password` y `provision` se
mantienen como alias de `set password`. Esta operación permite definir o rotar
la contraseña de `root@pam` sin reemplazar la identidad mesh existente; el valor
se solicita dos veces con eco desactivado y nunca se acepta en argv. `--pause` mantiene visible el resultado en una
consola interactiva. No se aceptan credenciales por argumentos, variables de
entorno ni entrada redirigida. Un error del servicio produce salida no cero.

`status` es lectura. `retry` no lleva secretos: reconcilia el payload y las
reglas de ingreso y vuelve a verificar; **puede reiniciar compute/gateway**.
`cancel` registra un checkpoint seguro, no destruye recursos ni garantiza
interrumpir instantáneamente una llamada síncrona en curso.

## Límites de confianza

```mermaid
flowchart LR
  CLI["Setup CLI / administrador"] -->|"Named Pipe + token Windows"| Agent["GnXHostAgent / LocalSystem"]
  Agent --> Identity["gnxnodesvc / perfil aislado"]
  Identity --> WSL["WSL2 gnx-node / Podman + systemd"]
  WSL --> Mesh["mesh / Tailscale"]
  WSL --> DNS["dns / Pi-hole"]
  WSL --> Gateway["gateway / Caddy"]
  Gateway --> App["app.gnx / PWA estática"]
  Gateway --> Compute["compute / HTTPS interno 8006"]
  User["Cliente autorizado del tailnet"] --> Gateway
```

- Único control privilegiado: `\\.\pipe\GnX.Platform.Control`, DACL explícita
  `SYSTEM`/`Administrators`, clientes remotos rechazados y token elevado validado.
- Operaciones versionadas: `GetProgress`, `Provision`, `JoinMesh`, `Retry`,
  `Cancel`. No shell arbitrario, API HTTP localhost ni operación `Deprovision`.
- La CLI verifica el proceso servidor contra `gnx-host-agent.exe` instalado
  junto a ella. El agente limita frames, valida versión, replay y transición.
- La PWA no abre pipes ni recoge credenciales del nodo. GitHub tiene una sola
  operación directa contra `api.github.com`: token clásico en memoria, comprobación
  de scopes `repo`/`workflow`, creación de repositorio privado e instalación del YAML
  elegido con consentimiento explícito. No usa protocolo nativo/UAC para GitHub,
  no importa `gh`, no inicia OAuth y no almacena sesiones. El YAML se proporciona
  como configuración; no se incrusta en JavaScript. Los secrets y la ejecución de
  Actions se administran en GitHub. La integración nativa previa permanece en la
  CLI por compatibilidad, pero no participa en este flujo de la PWA. Su Service Worker cachea solo assets, nunca `status.json`
  ni respuestas de compute.
- No se adoptan cuentas o distros ajenas. Se conserva el aislamiento del perfil
  y la credencial Windows protegida con DPAPI para el transporte existente.

## Secretos y estado

La contraseña Proxmox **la define la persona administradora** durante el alta;
no se descubre ni se genera una contraseña predeterminada desde la PWA.
La clave Tailscale también la proporciona esa persona.

Setup desactiva el eco antes de leer, restaura el modo de consola y borra los
buffers en memoria. Crea un bundle efímero en el directorio temporal del usuario,
aplica una DACL protegida antes de escribir valores y envía **solo la ruta**
por el pipe. El servicio consume/elimina el bundle y crea los archivos separados
`TS_AUTHKEY` y `PROXMOX_PASSWORD` para el contrato Linux. El cliente limpia el
bundle si la entrega falla; el servicio limpia su staging conocido al arrancar.

La clave de alta Tailscale se consume y elimina; una interrupción anterior a su
consumo puede requerir otra clave. La contraseña Proxmox sí se conserva en
`/var/lib/gnx-node/secrets/compute.env`, protegida con modo `0600`, para reinicios.
No se incluyen valores en Git, argv, URLs, logs, progreso ni estado público.
La CA privada queda protegida en `%ProgramData%\GnX\Node\state\ca`; solo se
presenta al usuario la ruta del certificado público.

La única fuente de estado Windows es `%ProgramData%\GnX\Node\state\`:
snapshot y journal sanitizados. El protocolo incluye estado/fase, secuencia,
identificador de operación, porcentaje, error, reinicio y acciones públicas.
Estados definidos: `NEW`, `WINDOWS_READY`, `REBOOT_REQUIRED`, `WSL_READY`,
`RUNTIME_READY`, `MESH_PENDING`, `NODE_READY`, `BLOCKED`, `FAILED`,
`RECOVERY_REQUIRED`. No se promete reanudación automática de pasos que necesiten
secretos después de reiniciar; se vuelve a ejecutar Setup para consultar y continuar.

`NODE_READY` exige verificación del runtime y publicación local de acceso en el
host Windows: confianza de CA y loopback HTTPS (incluido `rustfs.gnx`) se reconcilian con un código de
error estable antes de afirmar ready. Split DNS y confianza de CA siguen siendo
acciones humanas separadas solo para clientes tailnet externos. Un fallo no se
convierte en estado verde ni autoriza borrar identidad, distro, datos o credenciales.

## Runtime y contratos externos

Rust orquesta archivos; no contiene scripts, Quadlets, firewall ni templates
Linux incrustados. Se conserva la arquitectura de archivos externos:

| Fuente | Uso |
| --- | --- |
| `payload/host/.wslconfig` | Perfil dedicado; virtualización anidada |
| `payload/host/gnx-node-rootfs.tar` y `.sha256` | Rootfs sellado; verificar antes de importar |
| `payload/node/install.run` | `check` / `install` idempotentes, secretos por ruta |
| `payload/node/verify.run` | Verificador canónico, JSONL sanitizado y exit code |
| `payload/node/services/` | Quadlets por capacidad e imágenes por digest |
| `payload/node/network/ingress.nft` | Ingreso limitado a loopback y tailnet |
| `payload/node/gateway/routes.conf` | PWA estática y proxies HTTPS a compute y al sidecar RustFS |
| `apps/web-app/` | Assets estáticos ensamblados dentro del payload Linux |

RustFS conserva su sidecar Tailscale y TLS nativo con la CA privada del nodo.
`rustfs.gnx` resuelve al gateway existente; `rustfs.mesh.gnx` resuelve sólo al
sidecar para el backend de Caddy. Caddy consulta Pi-hole en loopback:5353 y valida
CA y nombre `rustfs.gnx` del backend. La política del tailnet debe permitir al
nodo gateway conectar a los servicios RustFS en TCP 443; no se abre el underlay.
La salud del workload exige también HTTPS del dashboard a través de Caddy.

Tailscale requiere TUN, **no KVM**. Compute requiere `/dev/kvm` funcional y
virtualización habilitada en firmware/Windows; no se simula disponibilidad.
El gateway valida la CA de backend de Proxmox; nunca usa TLS inseguro.
El puerto interno `8006` no se publica directamente a LAN/tailnet. No se abre
DNS público ni puertos del router. Un apagado global de WSL/reinicio del host
requiere consentimiento; no se usa para validar una actualización por sorpresa.

## Imagen OCI de compute

La imagen pertenece a `https://github.com/mayas-alas/tailnet-proxmox`, no al
workspace del producto. Ese repo actualiza `dockur/proxmox` y retira dos mecanismos:
la instalación de `pve-fake-subscription` y `blockLicense()`/su llamada, que
redirigía `shop.maurer-it.com` a loopback en el contenedor independientemente del
paquete. No añade activación falsa ni nag bypass. Repos apt no-subscription,
adaptaciones necesarias de contenedor y NAT upstream no son licencias falsas.
Su CI verifica el parche acotado, ausencia del paquete/caché falsa/bloqueo hosts,
boot de Proxmox y KVM antes de publicar el OCI exacto con Skopeo en
`ghcr.io/mayas-alas/tailnet-proxmox`, preservando y verificando el manifest digest.

Quetzalcoatl solo consume `Image=ghcr.io/mayas-alas/tailnet-proxmox@sha256:…` en
`payload/node/services/compute.container`. Podman/Quadlet gestiona la descarga y
el almacén de la identidad dedicada; no se incorpora un Containerfile, builder,
importer HTTP ni credenciales del usuario host. Las reglas de red, CA, secretos,
volúmenes, KVM y LXC del producto siguen siendo payload externo. Un digest nuevo
puede recrear el contenedor compute: la aceptación con VMs/LXC existentes exige
un upgrade explícito y no se deduce del smoke test aislado de la imagen.

MSI sólo instala el payload Windows. El snapshot read-only compara su marker con
la WSL y comprueba el release activo: un desfase es `RUNTIME_UPDATE_PENDING`, no
`NODE_READY`. Setup pide confirmación; `gnx upgrade --confirm` aplica sin re-enroll.
El lifecycle externo `compute/reconcile.run` descarga antes de sustituir Quadlet,
compara la identidad real del contenedor con la imagen fijada y valida paquete y
hosts. Un receipt de configuración sólo se escribe tras activación correcta;
fallos posteriores a copiar Quadlet no hacen que el Retry adopte el contenedor
viejo. Rust orquesta el script, no contiene referencias OCI ni reglas runtime.

## Actualización, retirada y organización

MSI/Burn mantienen UpgradeCodes estables y versiones crecientes. Cada versión nueva activa MajorUpgrade; ejecutar exactamente la misma versión es Repair, no se presenta como release nueva. Burn es la única entrada visible en Aplicaciones instaladas y oculta su MSI encadenado. La actualización conserva identidad, distro, datos, CA y credenciales. El MSI elimina los archivos
de Setup web de la versión anterior; no se mantienen dos mecanismos de alta.
`retry` aplica los assets nuevos al nodo existente sin introducir claves nuevas y
vuelve a reconciliar la confianza CA/acceso loopback del host antes de restaurar
`NODE_READY`.
Una desinstalación real es destructiva: después de detener el servicio elimina la
distro administrada, identidad/perfil dedicados, datos runtime, CA, credenciales,
confianza local, entradas `hosts` GnX, port-proxy loopback heredado y finalmente
estado. Las condiciones
`UPGRADINGPRODUCTCODE`/`WIX_UPGRADE_DETECTED` excluyen esta limpieza durante upgrades.
Una instalación genuinamente limpia también retira residuos de versiones antiguas
antes de iniciar el servicio, evitando publicar un Proxmox/LXC anterior.

El workspace es la raíz de este repositorio:

- `apps/host-agent/`: servicio y adaptadores Windows/Linux.
- `apps/setup/`: CLI y cliente del Named Pipe.
- `crates/control-protocol/`: mensajes y estados compartidos.
- `installer/`: `GnxNode.wixproj`, `Product.wxs`, `bundle/`, `scripts/`.
- `tests/`: contratos estáticos de instalación, runtime y seguridad; tests Rust
  junto al código.
- `target/`, `build/`: salidas generadas, fuera de Git.
- `releases/v*/`: sólo la última versión conserva notas, `SHA256SUMS.txt` y enlace al instalador verificado. Burn (con MSI embebido) se publica en esa carpeta del Space cuando cabe en su cuota; si el LFS histórico alcanza el límite, usar `gnx-labs-mx/gnx-node-releases` y enlazar su revisión inmutable verificada. No se duplica el MSI ni se purga historial. Binarios ignorados en Git fuente, publicación explícita. Las anteriores conservan Markdown en `docs/releases/`. `CHANGELOG.md` indexa cambios y notas históricas. No se sobrescriben artefactos ni se reescribe el historial Git/LFS.
- Tras las pruebas y notas de release, `installer/scripts/verify.ps1 -PublishRelease` prepara los artefactos nuevos y archiva el Markdown anterior en `docs/releases/` y retira sus directorios de `releases/`. La publicación en Hugging Face utiliza la WSL autenticada del usuario host.

La asignación de agentes y reglas de coordinación viven en `.AGENTS/`, no en
planes históricos duplicados aquí. En esta migración solo el coordinador ejecuta
pruebas, builds, instaladores y verificación elevada.

## Validación

```text
cargo test --workspace --locked
node tests/test-static-contracts.mjs
powershell -ExecutionPolicy Bypass -File installer/scripts/build.ps1
powershell -ExecutionPolicy Bypass -File installer/scripts/verify.ps1
```

Los tests no sustituyen aceptación real: alta en Windows limpio, entrega y
limpieza de secretos, denegación de usuario no administrador, reinicio, KVM/TUN,
cuatro servicios, Split DNS/CA y HTTPS deben comprobarse en un host autorizado.
Las capturas de prompts no prueban una inscripción completada; una actualización
sobre un nodo existente tampoco equivale a instalación limpia. Los resultados y
límites efectivamente observados se registran en las notas de cada release.

Referencias: documentación Microsoft de WSL/`.wslconfig`, Proxmox VE
Administration Guide (HTTPS `8006`) y WiX 5 (`ApprovedExeForElevation`).
