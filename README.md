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

Descarga la [candidata 0.0.18 para Windows x64](https://github.com/mayas-alas/quetzalcoatl-next/releases/tag/v0.0.18).
Incluye `GnxCoreSetup.exe`, `GnxCore.msi` y `SHA256SUMS.txt`. El repositorio es
privado: debes iniciar sesión en GitHub con una cuenta que tenga acceso.

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
correcta, se descargan desde **Actions → ejecución → Artifacts**. El workflow
publica en GitHub Releases solo si se activa `publish_release`, como pre-release
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
| `password` | Solicita sin eco la contraseña de Proxmox y aplica su configuración. |
| `stop` | Detiene los servicios propios y conserva datos e identidad. La recuperación del servicio Windows respeta esa parada. |
| `deploy <carpeta>` | Envía Quadlets admitidos por el pipe administrativo y crea un LXC no privilegiado nuevo. Una entrada idéntica ya aplicada no duplica el guest. |
| `update --check [catalogo.json]` | En las fuentes 0.0.20: compara las ocho imágenes con un catálogo aprobado por digest, sin descargar, reiniciar ni iniciar recursos detenidos. |
| `update --apply [catalogo.json]` | En las fuentes 0.0.20: descarga primero las candidatas y aplica cambios a las unidades existentes. Puede interrumpir acceso; no revierte migraciones. |

Se usa la ruta explícita porque el instalador no modifica `PATH`. No introduzcas
keys ni contraseñas en argumentos, navegador, logs o chat.

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
