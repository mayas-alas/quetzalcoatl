# GnX Core

Nodo privado para Windows con acceso HTTPS local y por Tailscale.
Una cuenta de servicio y una distribución WSL dedicadas ejecutan **Tailscale,
Pi-hole, Caddy y Proxmox VE** mediante systemd y Podman Quadlets.

## Funcionalidad disponible

- **https://app.gnx**: información del nodo y última fase de configuración.
- **https://compute.gnx**: Proxmox con autenticación nativa y TLS verificado.
- CLI para consultar, iniciar, detener y configurar las credenciales del nodo.
- Envío de una carpeta de Quadlets/configuración a un LXC nuevo mediante `deploy`.
- Desinstalación conservadora y reinstalación sin eliminar identidad ni datos.

La página local no requiere enrolar Tailscale ni esperar a que Proxmox esté listo.
La fase publicada no es un monitor de salud. El envío de Quadlets se realiza por
la CLI; la página no recibe archivos ni ejecuta operaciones administrativas.

## Instalación

Requiere Windows x64 con WSL2 y virtualización disponibles. Los instaladores
incluyen el filesystem Linux y las cuatro imágenes OCI del núcleo; no instalan
los prerrequisitos Windows que falten. No se garantiza cualquier host limpio.

Descarga la [candidata 0.0.22 para Windows x64](https://github.com/mayas-alas/quetzalcoatl/releases/tag/v0.0.22).
Incluye `GnxCoreSetup.exe`, `GnxCore.msi` y `SHA256SUMS.txt`. La release es pública;
no requiere una sesión GitHub para descargar esos archivos.

| Descarga | Cuándo usarla |
| --- | --- |
| **`GnxCoreSetup.exe` — recomendado** | Instalación interactiva mediante Burn, que contiene y ejecuta el MSI. |
| `GnxCore.msi` | Instalación directa mediante Windows Installer para administración o despliegue gestionado. |
| `SHA256SUMS.txt` | Verificar la integridad del instalador elegido. No es otro componente que instalar. |

Son dos vías al **mismo producto**, no dos pasos: elige una y usa su entrada de
mantenimiento en Windows. Ambos incluyen el mismo contenido offline, por eso
tienen tamaños similares. El EXE no descarga ni instala prerrequisitos Windows.
La CLI se instala con el producto; no se ofrece como aplicación portable.

No superpongas compilaciones distintas con el mismo número de versión ni cambies
entre MSI directo y Burn como si fueran pasos complementarios. Consulta
[mantenimiento y desinstalación](docs/uninstall.md) para los requisitos de retirada
y los datos que se conservan.

Los MSI y EXE se generan con la Action
[Windows installers](.github/workflows/installers.yml). Tras una ejecución
correcta y con `publish_release` activado, se descargan desde **GitHub Releases**.
El workflow entrega una pre-release
de prueba y con tag coincidente. Revisa `SHA256SUMS.txt` antes de usar
los archivos y ejecuta **uno** de los dos instaladores con permisos administrativos.
La candidata no tiene firma Authenticode; los hashes no sustituyen una firma de código.
Los enlaces automáticos «Source code» de GitHub son fuentes, no instaladores.

## Operación

En PowerShell como administrador:

```powershell
$gnx = "$env:ProgramFiles\GnX Core\gnx.exe"
& $gnx status
& $gnx start
& $gnx enroll
& $gnx password
& $gnx stop
```

| Comando | Resultado |
| --- | --- |
| `status` | Consulta el estado del target sin iniciar una WSL detenida. No certifica HTTPS ni salud de cada servicio. |
| `start` | Inicia el nodo preparado, sin instalar ni aplicar actualizaciones. |
| `enroll` | Solicita la key Tailscale sin eco y configura la mesh. Necesita conectividad con Tailscale. |
| `password` | Establece una nueva contraseña de `root@pam` en PVE, con confirmación sin eco, y conserva su hash en la configuración protegida. |
| `stop` | Detiene los servicios propios y conserva datos e identidad. La recuperación del servicio Windows respeta esa parada. |
| `deploy <carpeta>` | Envía Quadlets admitidos por el pipe administrativo y crea un LXC no privilegiado nuevo. Una entrada idéntica ya aplicada no duplica el guest. |
| `update --check [catalogo.json]` | En 0.0.23 sin catálogo consulta releases públicas, muestra la transacción y compara componentes. Con catálogo solo compara componentes localmente. No instala. |
| `update --apply [catalogo.json]` | En 0.0.23 sin catálogo selecciona la release pública y entrega MSI/Burn a un worker SCM; con catálogo solo aplica componentes. Bloquea instalación normal con guests activos. |
| `update --force` | En 0.0.23 recrea el nodo de la versión instalada, con inventario y confirmación interactiva de pérdida de datos y guests. |
| `update --apply --force` | Selecciona primero la release pública y recrea después el nodo. No reinicia Windows ni purga otras WSL. |

Se usa la ruta explícita porque el instalador no modifica `PATH`. No introduzcas
keys ni contraseñas en argumentos, la app informativa, logs o chat. Los formularios
de autenticación nativos de PVE se usan únicamente con HTTPS confiable.

### Contraseña de PVE

`gnx password` establece o cambia la contraseña de **`root@pam` en Proxmox**;
no pide la contraseña anterior ni cambia la cuenta Windows del operador o del
servicio. Con el nodo iniciado, ejecutar desde PowerShell elevado:

```powershell
$gnx = "$env:ProgramFiles\GnX Core\gnx.exe"
& $gnx password
```

1. Introducir la nueva contraseña: mínimo 12 bytes; 12 caracteres ASCII cumplen
   ese mínimo. Las fuentes 0.0.23 muestran asteriscos en consola interactiva,
   permiten borrar con retroceso y cancelar con Esc/Ctrl+C, sin mostrar la clave.
   La candidata 0.0.22 no muestra caracteres ni asteriscos.
2. Repetirla cuando se solicite. Una confirmación diferente rechaza la operación.
3. Comprobar el resultado del comando y acceder a `https://compute.gnx` como
   `root@pam` —o usuario `root` con realm **Linux PAM**— usando esa contraseña.

El comando persiste `PASSWORD_HASH` y actualiza root en compute, o inicia compute
si estaba detenido dentro de un nodo activo. No imprime ni permite recuperar la
contraseña guardada. Si el nodo está detenido, iniciarlo explícitamente antes.
Un timeout es resultado desconocido: revisar estado y diagnóstico antes de repetir.

Usar este mismo comando para cambios que deban persistir. Cambiar root solo desde
PVE modifica el contenedor, no la configuración protegida de GnX: al recrearlo,
el entrypoint vuelve a aplicar la credencial declarada. El éxito del comando no
sustituye una comprobación de login. Nunca pasar la contraseña como argumento.
La automatización por stdin conserva dos líneas, contraseña y confirmación,
desde una fuente protegida, sin máscaras ni eco en la salida del proceso.

Las actualizaciones se describen en [Actualización de componentes](docs/updates.md).
El catálogo no es un canal remoto ni descubre versiones nuevas automáticamente:
un administrador debe revisar y aportar los digests. Al iniciar se compara el
último catálogo aprobado, sin aplicación automática. La base WSL/LXC y el MSI/CLI
se mantienen por separado; un build no instala los nuevos comandos en el servicio.

El [contrato de despliegue](docs/deployment.md) define archivos, límites, recursos
y conservación. Las imágenes de aplicaciones requieren conectividad a sus
registros. La candidata 0.0.18 incluye `.network` y el caso fijo de
[servicios privados](stacks/private-services/README.md): bridge, HTTPS y clientes
OIDC de Pocket ID para RustFS y NetBird. No es una publicación genérica de cualquier
aplicación. Salud, login humano y conectividad entre peers se verifican por separado.

Para el primer acceso humano, abre `https://pocket.gnx/setup`, crea tu cuenta y
registra una passkey real. La preparación de OIDC no crea esa cuenta. STUN UDP
no atraviesa el portproxy TCP de Windows; no se acredita conectividad entre peers
externos por abrir el dashboard de NetBird.

## Acceso y conservación

El producto gestiona internamente los certificados y la confianza del host Windows;
no hay un comando ni un paso habitual de certificados. El acceso desde otros
dispositivos requiere Split DNS hacia el Pi-hole del nodo, las ACL correspondientes
y confianza en su CA privada. DNS no concede confianza TLS. Los requisitos técnicos
están en [Arquitectura](architecture.md).

La retirada normal solicita la parada del nodo y elimina archivos instalados,
servicio y acceso Windows propios. Conserva cuenta, perfil, WSL, CA, credenciales,
LXC y datos para reinstalar. **No es una purga ni libera todo el espacio usado**.
La acción requiere `GnXCore` en ejecución y su pipe disponible; un conflicto,
una operación ocupada o una parada no confirmada bloquean la retirada. Detener el
nodo con `gnx stop` no es lo mismo que detener el servicio Windows.

## Desarrollo y documentación

- [Compilación, inputs offline y GitHub Actions](docs/packaging.md)
- [Alcance y límites](scope.md)
- [Arquitectura](architecture.md)
- [Recorridos de usuario](historias-de-usuario.md)
- [Contrato para agentes](.AGENTS)
- [Mantenimiento y desinstalación](docs/uninstall.md)
- [Cambios de 0.0.18](releases/v0.0.18/CHANGELOG.md)

## Licencia

Código propio: [GNU AGPLv3](LICENSE), `AGPL-3.0-only`.
Las imágenes y dependencias de terceros conservan sus respectivas licencias.
