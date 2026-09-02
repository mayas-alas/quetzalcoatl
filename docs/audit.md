# Auditoría y huecos por cerrar

**Corte de investigación:** 2026-09-01  
**Base revisada:** rama `legacy` más documentación primaria actual

## Qué se conserva de legacy

- La cuenta dedicada de Windows es una buena frontera de propiedad para WSL y
  Podman Machine.
- systemd + Quadlet debe ser la autoridad del ciclo de vida.
- El nombre reservado de la máquina es `quetzalcoatl` y no se adopta una máquina
  ajena con el mismo nombre.
- Health checks, journal idempotente, imágenes por digest y secretos efímeros son
  decisiones correctas.
- KVM, TLS, red y compatibilidad deben ser gates observados, no supuestos.

Se descartan por ahora OpenTofu, runner LXC, workloads anidados, tray y el modelo
contradictorio donde Headscale era a la vez externo y creado dentro de Proxmox.

## Correcciones de entendimiento

| Impresión inicial | Hecho arquitectónico |
|---|---|
| “WSL dentro de Podman Machine” | En Windows, la Podman Machine está respaldada por una distribución WSL Fedora. |
| “Linux hace exactamente lo mismo” | Reutiliza Quadlets, pero Podman es nativo; una Podman Machine Linux sólo agregaría virtualización. |
| “Docktail registra el nodo de red” | `gnx-netd` se registra; Docktail consume su LocalAPI y el API del motor. |
| “Headscale puede quedar sólo en la mesh” | El control plane debe ser accesible antes del primer registro. |
| “Docktail + Headscale ya es una ruta válida” | Docktail requiere una API de Services; Headscale mantiene esa función como brecha abierta. |
| “Proxmox es otro contenedor normal” | Dockur/Proxmox es comunitario, privilegiado y requiere KVM/FUSE y persistencia. |

## Bloqueos P0

### P0.1 — Docktail y Headscale

Docktail anuncia servicios nativos, sincroniza definiciones con el control plane
y documenta credenciales OAuth/API externas. Headscale no lista esa API de
Services entre sus funciones soportadas y mantiene el issue `#2845` abierto con
la etiqueta de feature gap. También siguen abiertos los prerrequisitos de
certificados y publicación HTTPS administrados por el daemon.

Conservar la LocalAPI del daemon upstream no añade esas capacidades a Headscale.
Por tanto, la decisión sobre `gnx-netd` no resuelve este bloqueo.

**Consecuencia:** se puede empaquetar Docktail, pero no prometer la función ni
habilitarla por defecto. El gate `D-01` exige crear, resolver y alcanzar un
servicio real usando exclusivamente Headscale. Hoy se espera que falle y puede
requerir cambios coordinados en Headscale y Docktail.

### P0.2 — Multiinstalación: decisión cerrada, prueba pendiente

[ADR-0002](decisions/0002-mesh-identity-and-endpoint.md) fija un Headscale y un
`control_server` por mesh. La primera instalación usa `create`; las siguientes
usan `join`, generan identidades propias y dejan Headscale deshabilitado. Varias
instancias activas serían meshes distintas, no redundancia.

Falta demostrar `M-02` y `M-03` en Windows y Linux.

### P0.3 — Endpoint, DNS y TLS de Headscale

El proveedor DDNS actual usa una credencial de cuenta capaz de modificar todos
sus nombres; no ofrece alcance por instalación. Distribuirla crearía una llave
maestra en cada host. ADR-0002 exige un solo escritor y un servicio GNX que
entregue credenciales individuales con una asignación fija de FQDN.

Todavía falta implementar ese servicio y definir emisión/renovación TLS. En
Windows, WSL usa NAT por defecto; el acceso desde LAN requiere red mirrored y
firewall o un port proxy mantenido. También falta probar el hairpin desde la
propia Podman Machine al FQDN público.

**Gate `M-01`:** un dispositivo externo nuevo recibe la CA y resolución privada,
abre `/health` en `https://mesh.gnx`, valida TLS y se registra con
`gnx connect --control-server=https://mesh.gnx` sin edición manual del archivo
de hosts.

### P0.4 — KVM en Windows

Dockur declara Windows 11 con virtualización anidada. La instalación debe comprobar
Windows 11, virtualización de firmware, WSL actualizado y `/dev/kvm` dentro de la
Podman Machine antes de descargar o iniciar Proxmox.

**Gate `W-02`:** Proxmox arranca, conserva datos y puede crear una VM o LXC de
prueba después de reiniciar Windows. Sin esto, Windows no es un target soportado.

### P0.5 — Socket Podman entregado a Docktail

La API Podman concede toda la funcionalidad del motor y ejecución arbitraria
como el usuario del servicio. Un bind mount `:ro` protege el archivo montado, no
convierte las llamadas HTTP del socket en operaciones de lectura.

**Consecuencia:** sobre el motor rootful, comprometer Docktail equivale a poder
ejecutar código como root en el runtime. `D-02` exige un proxy o una frontera que
permita sólo las operaciones necesarias, o aceptar y documentar expresamente
ese riesgo. El socket permanece deshabilitado mientras tanto.

## Riesgos P1

- Falta crear flujos de seguridad para un primer release no dev or QA.

## Gates de aceptación mínimos

| ID | Evidencia exigida |
|---|---|
| `W-01` | El usuario estándar no puede listar ni operar la máquina de la cuenta dedicada; el servicio sí puede tras reboot. |
| `W-02` | `/dev/kvm`, `/dev/fuse`, Proxmox saludable y virtualización invitada real en Windows 11. |
| `W-03` | Un cliente LAN alcanza Headscale 443 a través de WSL y firewall después de reboot. |
| `L-01` | Instalación y recuperación en una distro limpia con systemd, cgroup v2, Podman 6+ y KVM. |
| `M-01` | Registro remoto contra el FQDN Headscale con TLS válido y key efímera. |
| `M-02` | Dos instalaciones conservan identidades distintas y el mismo control plane tras reinicios. |
| `M-03` | Un miembro no puede arrancar Headscale ni modificar el endpoint del control plane. |
| `E-01` | Existe un solo escritor por FQDN y el cliente no puede alterar su asignación. |
| `N-01` | `gnx-netd` pasa la matriz upstream, conserva LocalAPI y sincroniza actualizaciones de seguridad. |
| `D-01` | Docktail crea y sirve un servicio real usando Headscale, sin una API SaaS externa. |
| `D-02` | Docktail no obtiene control irrestricto del motor rootful, o el riesgo queda aceptado explícitamente. |
| `S-01` | Imágenes por digest, secretos fuera de logs/argv y permisos de persistencia verificados. |
| `S-02` | Config, entorno, logs, capturas y evidencia no contienen tokens ni URLs de actualización. |
| `R-01` | Backup y restore probado antes del primer upgrade destructivo. |
| `R-02` | Restore del controlador detrás del mismo FQDN sin dos escritores activos. |

## Preguntas que necesito cerrar contigo

1. ¿Windows 11 x86_64 con nested virtualization puede ser el mínimo oficial?
2. Si Docktail sigue bloqueado por upstream, ¿se pausa el release o aceptamos un
   reemplazo compatible con Headscale?
3. ¿La UI de Proxmox debe ser sólo local al principio o accesible desde la mesh?
4. ¿Qué distribución Linux es la primera referencia de aceptación?

## Fuentes primarias

- [Podman Machine](https://docs.podman.io/en/latest/markdown/podman-machine.1.html)
  y [machine init](https://docs.podman.io/en/latest/markdown/podman-machine-init.1.html)
- [Podman para Windows](https://github.com/podman-container-tools/podman/blob/main/docs/tutorials/podman-for-windows.md)
  y [release 6.1.0](https://github.com/podman-container-tools/podman/releases/tag/v6.1.0)
- [Quadlet](https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html)
- [Seguridad de la API Podman](https://docs.podman.io/en/latest/markdown/podman-system-service.1.html)
- [Red de WSL](https://learn.microsoft.com/en-us/windows/wsl/networking)
- [Root Zone Database de IANA](https://www.iana.org/domains/root/db)
- [Headscale: inicio](https://headscale.net/stable/usage/getting-started/),
  [contenedor](https://headscale.net/stable/setup/install/container/) y
  [funciones](https://headscale.net/stable/about/features/)
- [Brecha de Services en Headscale](https://github.com/juanfont/headscale/issues/2845)
- [Docktail](https://github.com/marvinvr/docktail)
- [Dockur/Proxmox](https://github.com/dockur/proxmox)
- [API DDNS actual](https://www.duckdns.org/spec.jsp) y
  [rotación del token](https://www.duckdns.org/faqs.jsp)
