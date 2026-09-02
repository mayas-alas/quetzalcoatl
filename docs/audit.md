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

### P0.2 — ¿Un Headscale por host o uno compartido?

La descripción actual instala Headscale en cada host. Eso crea meshes separadas:
un Windows y un Linux instalados independientemente no se descubren entre sí.

**Decisión requerida:** confirmar si Quetzalcoatl es una appliance autónoma por
host o si todos los hosts deben entrar a una misma mesh. Para una mesh compartida
debe existir un solo endpoint Headscale estable fuera del ciclo de vida de los
clientes, o una elección explícita de nodo controlador.

### P0.3 — Endpoint, DNS y TLS de Headscale

Falta definir quién entrega el FQDN, cómo apunta al host, quién emite/renueva el
certificado y cómo confían en él los clientes. En Windows, WSL usa NAT por defecto;
el acceso desde LAN requiere red mirrored y firewall o un port proxy mantenido.
También falta probar el hairpin desde la propia Podman Machine al FQDN público.

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
| `N-01` | `gnx-netd` pasa la matriz upstream, conserva LocalAPI y sincroniza actualizaciones de seguridad. |
| `D-01` | Docktail crea y sirve un servicio real usando Headscale, sin una API SaaS externa. |
| `D-02` | Docktail no obtiene control irrestricto del motor rootful, o el riesgo queda aceptado explícitamente. |
| `S-01` | Imágenes por digest, secretos fuera de logs/argv y permisos de persistencia verificados. |
| `R-01` | Backup y restore probado antes del primer upgrade destructivo. |

## Preguntas que necesito cerrar contigo

1. ¿Cada instalación debe tener su propia mesh o Windows y Linux deben compartir
   un solo Headscale?
2. ¿El endpoint Headscale será público en Internet, sólo LAN o accesible mediante
   un dominio privado y una CA corporativa?
3. ¿Windows 11 x86_64 con nested virtualization puede ser el mínimo oficial?
4. Si Docktail sigue bloqueado por upstream, ¿se pausa el release o aceptamos un
   reemplazo compatible con Headscale?
5. ¿La UI de Proxmox debe ser sólo local al principio o accesible desde la mesh?
6. ¿Qué distribución Linux es la primera referencia de aceptación?

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
