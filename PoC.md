He tenido problemas para aterrizar el PoC funcional desde un exe y bien definido:
---
Concern	Decision
Setup executable	WiX Toolset 5 Burn
Windows package	WiX Toolset 5 MSI
Host VM provider	WSL2
Prerequisite orchestration	Burn detects/enables WSL2 and chains Podman
Windows service supervision	WinSW
Privileged control process	gnx-service.exe in Rust
Operator CLI	gnx.exe in Rust
Desktop UX	Tauri Tray
Local IPC	Windows Named Pipes with ACL and kernel-derived caller identity
Local Linux runtime	Podman Machine managed Fedora
Managed machine identity	Explicit named machine owned by the dedicated Windows account
Persistent local workloads	systemd Quadlets
Private networking	Tailscale SaaS permanently for Quetzalcoatl/PVE hosts; Headscale only for a separate internal services overlay inside PVE
Infrastructure engine	OpenTofu
Infrastructure target	Proxmox API
Mutable Windows data	%ProgramData%\Quetzalcoatl
Windows secrets	DPAPI / Credential Manager
Updates	MSI major upgrades plus versioned runtime migrations
Node roles	Explicit controller or member
Infrastructure writer	Controller-only OpenTofu single writer
---
**Installer decision matrix**
---
Detected state	Action
WSL2 absent	Enable WSL2, record resume state, reboot and continue
WSL2 healthy, Podman absent	Install pinned Podman MSI
Podman compatible, managed machine absent	Create the named machine
Managed machine healthy	Reuse it
Controller fresh install	Initialize cluster and permit bootstrap OpenTofu
Member fresh install	Consume invitation and join; do not deploy singleton services
Product-only upgrade	Upgrade MSI and preserve machine/state
Podman incompatible	Controlled CLI and machine compatibility update
Runtime profile outdated	Apply versioned migrations
Normal uninstall	Remove product files; preserve managed data
Explicit purge	Remove machine/state only after destructive confirmation
---
Installer UX
---

Burn uses a branded, restrained interface:
License Agreement check 

Checking Windows
Preparing WSL2
Installing Podman
Installing Quetzalcoatl
Preparing managed runtime
Enrolling node / initializing controller
Starting service
Reboot if necesary 
Ready
---
Basic cli interactions for resume basic as

**Commands and scope**
Command	Purpose
gnx status [--json]	Global service, runtime, network, cluster and infrastructure status
gnx runtime status	Podman Machine, systemd and Quadlet health
gnx runtime logs	Retrieve bounded local runtime logs
gnx runtime logs	Retrieve bounded local runtime logs
gnx cluster status|members|health	Proxmox membership, quorum and private connectivity

Managed Windows identity 
Podman is operated through a dedicated Windows identity rather than the interactive user. The managed account owns one explicitly named Podman Machine, for example quetzalcoatl, so Quetzalcoatl never takes ownership of unrelated user machines.

8. Managed Fedora runtime
Windows owns the WSL2 provider. Podman creates and owns the Fedora machine. Quetzalcoatl owns only a versioned runtime profile applied inside that machine.

Windows → WSL2 provider
Podman  → Fedora machine lifecycle
GNX     → configuration, migrations, systemd and Quadlets

Managed Fedora runtime
Windows owns the WSL2 provider. Podman creates and owns the Fedora machine. Quetzalcoatl owns only a versioned runtime profile applied inside that machine.

Windows → WSL2 provider
Podman  → Fedora machine lifecycle
GNX     → configuration, migrations, systemd and Quadlets

**Network invariants**
no Proxmox port is published to Windows;
Tailscale Serve exposes approved application endpoints only;
OpenTofu reaches the Proxmox API through private networking;
Corosync and SSH use explicitly authorized peer addresses;
cluster-control traffic is not routed through Tailscale Config file; 
firewall rules limit Corosync UDP 5405–5412 and SSH to authorized peers.

Remote platform services 
Garage, Headscale, Forgejo and Forgejo Runner are remote platform services placed inside Proxmox guests by the controller through OpenTofu. They are not node-local Quadlets and a member installation never recreates them.

Operational scenario index 
 
Host preflight
→ WSL2/Podman preparation
→ MSI installation
→ managed machine creation
→ local runtime convergence
→ Proxmox or cluster initialization 
→ OpenTofu bootstrap
→ Garage + Headscale + Forgejo + Runner

---
Un solo proceso Windows privilegiado.
Un solo escritor de infraestructura: OpenTofu.
Tres responsabilidades locales materializadas por Quadlets y unidades de soporte.
Cuatro servicios remotos.
CLI y tray sin privilegios.
---

Reglas de ownership
Windows owns WSL2.

Podman owns the managed Fedora machine.

GNX owns the runtime profile applied inside Fedora.

MSI owns immutable Windows product files.

ProgramData owns mutable product state.

OpenTofu owns Proxmox infrastructure mutations.

GNX CLI
gnx.exe es la interfaz operativa para usuarios técnicos, automatización y agentes de IA.

La CLI no realiza operaciones privilegiadas directamente. Envía comandos al servicio por Named Pipe.

Quadlets locales
Los Quadlets forman el plano de control local.

gnx-node.pod
pod compartido para Proxmox y Tailscale;
namespace de red compartido;
orden y dependencias mediante systemd.
proxmox.container
ejecuta la imagen custom de Proxmox;
persistencia de datos;
configuración de networking;
configuración de storage;
health checks;
creación y unión de clúster;
API accesible únicamente por red privada;
sin puertos publicados en Windows.
tailscaled.container
conecta el runtime a la tailnet;
comparte red con Proxmox;
mantiene estado persistente;
expone Proxmox mediante Tailscale Serve;
proporciona HTTPS/TCP privado;
evita port forwarding hacia Windows;
permite comunicación privada entre OpenTofu y Proxmox.
opentofu.image
referencia la imagen del runner;
no ejecuta un daemon permanente;

Imagen custom de Proxmox

https://github.com/mayas-alas/tailnet-proxmox/blob/master/Dockerfile
docker pull ghcr.io/mayas-alas/tailnet-proxmox:latest

Features
imagen OCI propia;
versionado por digest;
arranque idempotente;
configuración de interfaces;
configuración de storage;
health checks;
persistencia;
scripts de cluster init;
scripts de cluster join;
consulta de cluster status;
roles controller/member;
API privada mediante Tailscale;

Networking privado
Features
Tailscale como red privada inicial.
Tailscale Serve para HTTPS.
Sin puertos publicados al host Windows.
Certificados de tailnet.
Acceso privado a UI y API de Proxmox.
Acceso privado OpenTofu → Proxmox.
Red compartida Proxmox/Tailscale.
Corosync limitado a peers autorizados.
SSH limitado a la tailnet.
Sin broadcast discovery.
Perfiles controller y member.
Headscale no participa en el overlay de hosts Proxmox; queda reservado al clúster interno de servicios dentro de PVE.

Estos servicios son desplegados por el controller mediante OpenTofu dentro de guests Proxmox, son enviados a lxc con los archivos de configuracion dentro del lxc hay docker compose para correr estos servicios. No son Quadlets locales de Podman Machine y una instalación member no los vuelve a crear.

Garage S3
almacenamiento compatible con S3;
backend de OpenTofu;
buckets;
credenciales protegidas;
persistencia;
health checks;
bootstrap idempotente;
almacenamiento de artefactos y backups.
Headscale
control server Tailscale autohospedado;
políticas ACL;
registro de nodos;
persistencia;
singleton inicial;
migraciones;
protección contra replay;
autoridad exclusiva sobre un overlay interno de servicios; sin transición de los hosts Proxmox.
Forgejo
repositorios Git;
organizaciones y usuarios;
API;
webhooks;
Actions;
persistencia;
configuración idempotente;
health checks;
acceso privado.
Forgejo Runner
registro automático;
ejecución de CI;
jobs aislados;
tokens protegidos;
persistencia de configuración;
construcción de imágenes OCI;
build, test y release de Quetzalcoatl.

los ejemplos provienen de: https://github.com/tailscale-dev/video-code-snippets/tree/main/2026/2026-03-s3-garage/docker

configs:
  mesh-serve:
    content: |
      {"TCP":{"443":{"HTTPS":true}},"Web":{"$${TS_CERT_DOMAIN}:443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:8642"}}}},"AllowFunnel":{"$${TS_CERT_DOMAIN}:443":false}}

services:
  mesh:
    image: tailscale/tailscale:v1.98.4
    hostname: ${TS_HOSTNAME:-agent-runtime}
    environment:
      TS_AUTHKEY: ${TS_AUTHKEY:?TS_AUTHKEY is required}
      TS_STATE_DIR: /var/lib/tailscale
      TS_SERVE_CONFIG: /config/serve.json
    volumes:
      - ./mesh-state:/var/lib/tailscale
    configs:
      - source: mesh-serve
        target: /config/serve.json
    devices:
      - /dev/net/tun:/dev/net/tun
    cap_add:
      - NET_ADMIN
      - SYS_MODULE
    restart: unless-stopped

  agent-runtime:
    image: ${AGENT_RUNTIME_IMAGE:-nousresearch/hermes-agent:latest}
    container_name: agent-runtime
    command: gateway run
    network_mode: service:mesh
    depends_on:
      - mesh
    restart: unless-stopped
    environment:
      API_SERVER_ENABLED: ${API_SERVER_ENABLED:-true}
      API_SERVER_HOST: ${API_SERVER_HOST:-0.0.0.0}
      API_SERVER_KEY: ${API_SERVER_KEY:?API_SERVER_KEY is required}
      API_SERVER_CORS_ORIGINS: ${API_SERVER_CORS_ORIGINS:-http://localhost:3000}
      HERMES_DASHBOARD: ${HERMES_DASHBOARD:-1}
      HERMES_DASHBOARD_BASIC_AUTH_USERNAME: ${HERMES_DASHBOARD_BASIC_AUTH_USERNAME:?HERMES_DASHBOARD_BASIC_AUTH_USERNAME is required}
      HERMES_DASHBOARD_BASIC_AUTH_PASSWORD: ${HERMES_DASHBOARD_BASIC_AUTH_PASSWORD:?HERMES_DASHBOARD_BASIC_AUTH_PASSWORD is required}
      HERMES_DASHBOARD_BASIC_AUTH_SECRET: ${HERMES_DASHBOARD_BASIC_AUTH_SECRET:?HERMES_DASHBOARD_BASIC_AUTH_SECRET is required}
    volumes:
      - ./data:/opt/data


configs:
  mesh-serve:
    content: |
      {"TCP":{"443":{"HTTPS":true},"2222":{"TCPForward":"127.0.0.1:22"}},"Web":{"$${TS_CERT_DOMAIN}:443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:3000"}}}},"AllowFunnel":{"$${TS_CERT_DOMAIN}:443":false}}

services:
  mesh:
    image: tailscale/tailscale:v1.98.4
    hostname: ${TS_HOSTNAME:-git-service}
    environment:
      TS_AUTHKEY: ${TS_AUTHKEY:?TS_AUTHKEY is required}
      TS_STATE_DIR: /var/lib/tailscale
      TS_SERVE_CONFIG: /config/serve.json
    volumes:
      - ./mesh-state:/var/lib/tailscale
    configs:
      - source: mesh-serve
        target: /config/serve.json
    devices:
      - /dev/net/tun:/dev/net/tun
    cap_add:
      - NET_ADMIN
      - SYS_MODULE
    restart: unless-stopped

  forgejo:
    image: ${FORGEJO_IMAGE:-codeberg.org/forgejo/forgejo:15}
    container_name: forgejo
    network_mode: service:mesh
    depends_on:
      - mesh
    environment:
      USER_UID: "1000"
      USER_GID: "1000"
      FORGEJO__server__DOMAIN: ${TS_CERT_DOMAIN}
      FORGEJO__server__ROOT_URL: https://${TS_CERT_DOMAIN}/
      FORGEJO__server__SSH_DOMAIN: ${TS_CERT_DOMAIN}
      FORGEJO__server__SSH_PORT: "2222"
      FORGEJO__actions__ENABLED: "true"
      FORGEJO__database__DB_TYPE: sqlite3
      FORGEJO__database__PATH: /data/gitea/forgejo.db
      FORGEJO__security__INSTALL_LOCK: "true"
      FORGEJO__security__SECRET_KEY: ${FORGEJO_SECRET_KEY:?FORGEJO_SECRET_KEY is required}
      FORGEJO__security__INTERNAL_TOKEN: ${FORGEJO_INTERNAL_TOKEN:?FORGEJO_INTERNAL_TOKEN is required}
    volumes:
      - ./data:/data
    restart: unless-stopped


flowchart TB
    Setup["QuetzalcoatlSetup.exe<br/>WiX 5 Burn"]
    Preflight["Preflight<br/>OS · CPU · virtualization · disk · reboot"]
    WSL{"WSL2 ready?"}
    EnableWSL["Enable WSL + Virtual Machine Platform"]
    Resume["Persist resume state<br/>reboot if required"]
    Podman{"Podman compatible?"}
    PodmanPkg["Install / upgrade Podman MSI"]
    MSI["Quetzalcoatl.msi"]
    Role{"Installation role"}
    Controller["Controller initialization"]
    Member["Member script enrollment"]

    subgraph MsiOwned["MSI-owned product files"]
        CLI["gnx.exe"]
        Service["gnx-service.exe"]
        WinSW["WinSW"]
        Tray["QuetzalcoatlTray.exe"]
        Payload["Versioned runtime payload"]
    end

    subgraph Mutable["Mutable product state"]
        ProgramData["%ProgramData%\Quetzalcoatl"]
        Secrets["DPAPI / Credential Manager"]
        RuntimeAccount["Managed runtime account"]
    end

    Machine["Named Podman Machine<br/>Fedora via WSL2"]
    Runtime["systemd + Quadlets"]
    Verify["Role-aware health verification"]

    Setup --> Preflight --> WSL
    WSL -->|No| EnableWSL --> Resume --> Preflight
    WSL -->|Yes| Podman
    Podman -->|No| PodmanPkg --> MSI
    Podman -->|Yes| MSI

    MSI --> CLI
    MSI --> Service
    MSI --> WinSW
    MSI --> Tray
    MSI --> Payload
    MSI --> ProgramData
    MSI --> RuntimeAccount
    Service --> Secrets

    MSI --> Role
    Role -->|controller| Controller --> Machine
    Role -->|member| Member --> Machine
    RuntimeAccount --> Machine
    Machine --> Runtime --> Verify



    flowchart TB
    subgraph Controller["Controller Windows installation"]
        CCLI["GNX CLI / Tray"]
        CService["gnx-service"]
        CRuntime["Managed Fedora runtime"]
        Writer["OpenTofu single writer"]
        CProxmox["Controller Proxmox node"]
    end

    Invite["Signed · expiring · single-use invite"]
    TailSaaS["Tailscale SaaS host overlay"]

    subgraph Member["Member Windows installation"]
        MCLI["GNX CLI / Tray"]
        MService["gnx-service"]
        MRuntime["Managed Fedora runtime"]
        MProxmox["Member Proxmox node"]
    end

    subgraph Platform["Controller-managed services in Proxmox"]
        Garage["Garage"]
        Headscale["Headscale"]
        ServiceOverlay["Separate internal services overlay"]
        Forgejo["Forgejo"]
        Runner["Runner"]
    end

    CCLI --> CService --> CRuntime --> CProxmox
    CService --> Writer
    Writer --> Platform

    CService --> Invite --> MService
    MCLI --> MService --> MRuntime --> MProxmox
    CProxmox --> TailSaaS
    MProxmox --> TailSaaS
    TailSaaS -->|"authorized API · SSH · Corosync"| CProxmox
    Headscale -.-> ServiceOverlay
    MProxmox -->|"authorized cluster membership"| CProxmox
    MService -. "status / requested operations" .-> CService

    Release outputs:

QuetzalcoatlSetup.exe
Quetzalcoatl.msi
gnx.exe
gnx-service.exe
QuetzalcoatlTray.exe
OCI image digests
SBOM
SHA-256 checksums
release notes
Supply-chain controls:

pinned Rust, Node and WiX versions;
locked Rust and Node dependencies;
OCI digests;
signed executables and installers;

run docker inside lxc

#!/usr/bin/env sh
set -eu

if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
  systemctl enable --now docker
  exit 0
fi

export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y ca-certificates curl gnupg fuse-overlayfs
install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc
chmod a+r /etc/apt/keyrings/docker.asc

. /etc/os-release
ARCH=$(dpkg --print-architecture)
printf 'deb [arch=%s signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian %s stable\n' "$ARCH" "$VERSION_CODENAME" > /etc/apt/sources.list.d/docker.list
apt-get update
apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

install -d /etc/docker
cat > /etc/docker/daemon.json <<'EOF'
{
  "storage-driver": "fuse-overlayfs"
}
EOF

systemctl enable --now docker
docker info >/dev/null
docker compose version
test -c /dev/net/tun


## Exolicacion de lo que se espera obtener

Se busca que el PoC no sea mas que el minimo funcionable para tener el instalador wide del sistema y el CLI

Al instalar el exe que buscamos correr en este PoC inicie la instalacion, lo primero va a ser la aceptacion del agreement y despues el instalador debe tener checks para poner si instala forgejo y Garage, tambien tiene que tener espacio para poner la tailnet ej tetra-balance.ts.net ademas de el client_id y auth_key, la contraseña de proxmox que vene desde el build con PASSWORD=root, asi que se cambio el usuario al momento de la instalacion, el instalador por la cli de tailscale que tiene instalado en el podman prodra ver si es la única máquina o ya exite otra con tailscale status, si existe ya una maquina en tailscale el nodo que se va instalar es member, si no exixte ninguna maqui sera contrller, asi es como se define, los flujos siguientes estan descritos en los diagramas, si la maquina no tiene wsl2 el installer le hace wsl install, despues le instala la cli de podman, al terminar esto ya se podra contar con el fedora y poner la custom podman machine, la podman machcine tiene los 3 servicios por el quadlet, el proxmox custom que puse en el repo + el tailscale con el serve.conf  + el open tofu para poner los servicios dentro del proxmox, tambien deje el ejemplo del repo de tailscale. si es member va  a hacer la misma rutina que que esta explicada pero tiene que una vez corriendo el proxmox correr una rutina dentro del pve con pvecm para agregar el pve controller que ya estan en red correctamente con los puertos udp y ssh que necesita para clusterizar, ya esa definicion esta en el serve.conf de tailscale, otra cosa si el usuario puso garage y forgejo tiene que solocitar las keys para correr con el sidecar de tailsacale, tambien puse los compose de ejemplo que viven dentro de lxc

El PoC se tiene que mantener minimo para poder lograr tener una beta en el primer intento, no se debe de cargar con casos de uso futuros ni casos alternos o pruebas o cualquier otra cosa que amplie el alcance.

Lo mejor es simplificar al maximo para poder hacer un exe ejecutable que vaya siendo incremental, se espera en una primera tirada la base de codigo completa son correr benchmarks, pruebas etc para entregar puntualmete el codigo y despues de eso tratar de contruir los artefactos y correr en host como primeras pruebas
