# GnX Core

Nodo privado para Windows con acceso HTTPS local y por Tailscale.
Una cuenta de servicio y una distribución WSL dedicadas ejecutan **Tailscale,
Pi-hole, Caddy y Proxmox VE** mediante systemd y Podman Quadlets.

## Funcionalidad disponible

- **https://app.gnx**: información del nodo y última fase de configuración.
- **https://compute.gnx**: Proxmox con autenticación nativa y TLS verificado.
- CLI para consultar, iniciar, detener y configurar las credenciales del nodo.
- Desinstalación conservadora y reinstalación sin eliminar identidad ni datos.

La página local no requiere enrolar Tailscale ni esperar a que Proxmox esté listo.
La fase publicada no es un monitor de salud. El despliegue de archivos/Quadlets
hacia LXC todavía no está implementado.

## Instalación

Requiere Windows x64 con WSL2 y virtualización disponibles. Los instaladores
incluyen el filesystem Linux y las cuatro imágenes OCI del núcleo; no instalan
los prerrequisitos Windows que falten. No se garantiza cualquier host limpio.

Los MSI y EXE se generan con la Action
[Windows installers](.github/workflows/installers.yml). Tras una ejecución
correcta, se descargan desde **Actions → ejecución → Artifacts**. El workflow no
publica en GitHub Releases solo si se activa `publish_release`, como pre-release
de prueba y con tag coincidente. Revisa `SHA256SUMS.txt` antes de usar
los archivos y ejecuta **uno** de los dos instaladores con permisos administrativos.

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

Se usa la ruta explícita porque el instalador no modifica `PATH`. No introduzcas
keys ni contraseñas en argumentos, navegador, logs o chat.

## Acceso y conservación

El producto gestiona internamente los certificados y la confianza del host Windows;
no hay un comando ni un paso habitual de certificados. El acceso desde otros
dispositivos requiere Split DNS hacia el Pi-hole del nodo, las ACL correspondientes
y confianza en su CA privada. DNS no concede confianza TLS. Los requisitos técnicos
están en [Arquitectura](architecture.md).

Desinstalar detiene el nodo y retira el servicio y el acceso Windows propios.
Conserva cuenta, perfil, WSL, CA, credenciales y datos para reinstalar. **No es una
purga**. Un conflicto de propiedad o una parada no confirmada bloquean la retirada.

## Desarrollo y documentación

- [Compilación, inputs offline y GitHub Actions](docs/packaging.md)
- [Alcance y límites](scope.md)
- [Arquitectura](architecture.md)
- [Recorridos de usuario](historias-de-usuario.md)
- [Contrato para agentes](.AGENTS)
- [Cambios de 0.0.14](releases/v0.0.14/CHANGELOG.md)

## Licencia

Código propio: [GNU AGPLv3](LICENSE), `AGPL-3.0-only`.
Las imágenes y dependencias de terceros conservan sus respectivas licencias.
