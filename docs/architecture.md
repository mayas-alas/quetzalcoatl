# Arquitectura base

**Estado:** propuesta auditable, todavía no implementable de extremo a extremo  
**Casos soportados:** Windows 11 x86_64 y Linux x86_64 con systemd  
**Corte:** 2026-09-01

## Decisión principal

Las diferencias terminan en la preparación del host. Windows necesita una
Podman Machine respaldada por WSL; Linux usa el motor nativo. Después de ese
límite, ambos reciben el mismo conjunto versionado de Quadlets.

```mermaid
flowchart LR
    subgraph W["Windows 11 x86_64"]
        EXE["Instalador EXE elevado"] --> ACCOUNT["Cuenta local dedicada"]
        ACCOUNT --> CLIW["Podman CLI 6+"]
        CLIW --> PM["Podman Machine quetzalcoatl"]
        WSL["WSL 2"] -->|"provider"| PM
        PM --> FSW["Fedora de Podman + systemd"]
    end

    subgraph L["Linux x86_64"]
        BIN["Instalador Linux"] --> CHECK["Gate systemd + cgroup v2 + KVM"]
        CHECK --> CLIL["Podman 6+ nativo"]
        CLIL --> FSL["systemd del host"]
    end

    MANIFESTS["Mismos Quadlets versionados"] --> FSW
    MANIFESTS --> FSL
    FSW --> RUNTIME["Contrato de runtime común"]
    FSL --> RUNTIME
```

No se usa Podman Machine en Linux: es opcional allí y añadiría otra VM justo
donde Proxmox necesita acceso directo y comprobable a `/dev/kvm`.

## Modelo Windows

1. El usuario abre el EXE y acepta UAC.
2. El instalador crea `quetzalcoatl-runtime`, cuenta local sin inicio de sesión
   interactivo, y prepara su perfil con ACL exclusivas.
3. Instala Podman 6+ en alcance de máquina y habilita o actualiza WSL 2.
4. Ejecuta `podman machine init --provider wsl --rootful quetzalcoatl` bajo la
   identidad dedicada. La máquina resultante es una distribución Fedora WSL con
   systemd; no es una distro WSL adicional dentro de otra VM.
5. Un Windows Service ejecutado como la misma cuenta arranca la máquina y
   reconcilia los Quadlets. El usuario interactivo sólo usa una interfaz local
   acotada; no recibe el socket Podman ni las credenciales.

La separación protege frente al usuario estándar, no frente a un administrador
local. La creación y recuperación de una Podman Machine desde una cuenta de
servicio sigue siendo un gate físico obligatorio.

## Modelo Linux

1. El instalador obtiene privilegios administrativos.
2. Falla sin mutar si PID 1 no es systemd, cgroup v2 no está activo, no existe
   `/dev/kvm`, falta `/dev/fuse` o la arquitectura no es x86_64.
3. Instala o valida Podman 6+. El socket API de sistema permanece cerrado hasta
   que exista una decisión segura para Docktail.
4. Copia Quadlets a `/etc/containers/systemd/` y datos persistentes a
   `/var/lib/quetzalcoatl/`.
5. systemd inicia y recupera el runtime después de cada reboot.

El runtime es rootful en los dos hosts porque Dockur/Proxmox requiere un
contenedor privilegiado y acceso a KVM. Los demás contenedores conservan sólo
los permisos y montajes indispensables.

## Runtime común

```mermaid
flowchart TB
    DNS["FQDN estable + TLS confiable"] --> HS["Headscale"]
    HS --> HSD[("configuración + SQLite")]
    HS -->|"health OK"| BOOT["Bootstrap one-shot"]
    BOOT -->|"pre-auth key efímera"| TS["gnx-netd"]
    TS -->|"login-server = URL de Headscale"| HS
    TS --> TSD[("identidad y estado de red")]

    PODMAN["API Podman rootful"] -.-> DT["Docktail condicionado"]
    TS -.->|"socket local"| DT
    DT -.->|"bloqueado: Tailscale Services"| HS

    KVM["/dev/kvm + /dev/fuse"] --> PVE["Dockur/Proxmox privilegiado"]
    PVE --> PVED[("configuración + discos")]
    LOCAL["Acceso administrativo local"] -->|"8006/TLS"| PVE
    REMOTE["Clientes externos"] -->|"443/TLS, fuera de la mesh"| HS
```

Headscale debe ser alcanzable antes de que exista la mesh; por eso su endpoint
443 no puede depender de Docktail ni de Tailscale. Proxmox queda limitado al host
hasta resolver la exposición por mesh.

Docktail no registra el nodo: observa el socket del motor y usa la LocalAPI de
`gnx-netd`. Su Quadlet puede estar empaquetado, pero no debe habilitarse por
defecto hasta pasar `D-01`. Montar el socket Podman con `:ro` no limita los
métodos de su API: antes de habilitarlo también debe pasar `D-02`, con una
restricción comprobable o una aceptación explícita del control total del motor.

`gnx-netd` será un fork mínimo del `tailscaled` oficial. Escucha en
`/run/gnx/netd.sock` y conserva compatibilidad LocalAPI. Dentro del directorio
compartido se publica `tailscaled.sock -> netd.sock` para herramientas que aún
esperan el nombre upstream. Esto es compatibilidad local; no concede a
Headscale capacidades nuevas de control plane.

## Orden de convergencia

```mermaid
sequenceDiagram
    participant O as Orquestador
    participant H as Headscale
    participant T as gnx-netd
    participant P as Proxmox
    participant D as Docktail

    O->>O: Validar host, Podman, cgroup v2, KVM y puertos
    O->>H: Instalar configuración y arrancar Quadlet
    H-->>O: Health y TLS válidos
    O->>H: Crear usuario y pre-auth key
    O->>T: Arrancar con login-server y key efímera
    T-->>O: Nodo registrado y conectado
    O->>P: Arrancar Quadlet privilegiado
    P-->>O: API 8006 saludable
    O-xD: Mantener deshabilitado mientras falle D-01
    O->>O: Reportar BLOCKED, nunca READY completo
```

## Fronteras y persistencia

| Recurso | Persistencia | Exposición inicial |
|---|---|---|
| Headscale | configuración, base SQLite, claves TLS | `443/tcp` mediante FQDN estable |
| gnx-netd | identidad, claves y preferencias del nodo | puerto mesh según su modo de red |
| Docktail | sin estado propio | ninguna; condicionado |
| Proxmox | `/var/lib/vz` y `/var/lib/pve-cluster` | `127.0.0.1:8006` o equivalente host-local |

El secreto de bootstrap se crea después de que Headscale esté sano, se entrega a
`gnx-netd` sin argumentos ni logs y se elimina después del registro. Las
imágenes se fijan por digest. Las claves TLS, la base de Headscale y los discos
de Proxmox nunca viven en la capa escribible del contenedor.

## Árbol objetivo

```text
quetzalcoatl/
├── README.md
├── docs/
│   ├── architecture.md
│   ├── audit.md
│   └── decisions/
│       └── 0001-network-daemon.md
├── src/
│   ├── cli/                       # interfaz pública
│   ├── host/
│   │   ├── windows/               # UAC, cuenta, WSL, service
│   │   └── linux/                 # systemd, paquetes, KVM
│   ├── runtime/                   # convergencia común
│   └── state/                     # journal y estados observados
├── runtime/
│   ├── quadlets/
│   │   ├── quetzalcoatl.network
│   │   ├── headscale.container
│   │   ├── gnx-netd.container
│   │   ├── docktail.container     # condicionado por D-01
│   │   └── proxmox.container
│   └── headscale/                 # plantilla config y policy mínima
├── packaging/
│   ├── windows/
│   └── linux/
└── tests/
    ├── contract/
    └── physical/
```

OpenTofu, automatización de LXC, tray, UI y catálogo de workloads no pertenecen
a esta primera arquitectura. Se agregarán sólo con una decisión explícita y un
caso de uso verificable.
