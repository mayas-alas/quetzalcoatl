# GNX — Propuesta de Arquitectura Completa

## 🎯 Objetivo del Proyecto

Crear una plataforma que:

1. **En Windows:** Gestiona un usuario dedicado, crea una máquina WSL con Podman, instala cuadlets (Quadlets) y expone una CLI al host Windows.
2. **En Linux nativo:** Instala solo el binario de Linux (como AppImage), que pone la CLI + servicio con sockets para datos sensibles.

**Principio rector:** Menos código, máxima claridad conceptual. Los diagramas Mermaid son el mapa principal.

---

## 🗺️ Diagrama de Arquitectura Global

```mermaid
flowchart TB
    subgraph Windows["🪟 Windows Host"]
        direction TB
        user[Usuario Operador] -->|CLI| cli[gnx CLI]
        cli --> broker[[Named pipe: GNX]]
        broker --> service[GNXRuntime Service]
        
        subgraph "GNXRuntime (Cuenta Dedicada gnx-runtime)"
            direction TB
            wsl[WSL2: Ubuntu 24.04]
            subgraph "En WSL"
                podman[Podman + systemd]
                services["Quadlets: access | dns | compute | controller"]
                tailscale[Tailscale bridge]
            end
        end
        
        wsl -->|volume mount| service
    end
    
    subgraph Linux["🐧 Linux Nativo"]
        direction TB
        user2[Usuario Operador] -->|CLI| cli_linux["gnx CLI (AppImage)"]
        cli_linux --> service_linux[GNXService]
        
        subgraph "GNXService"
            direction TB
            socket[Unix Socket /run/gnx/gnx.sock]
            subnet[Red virtual bridge0]
            pods[Podman Pods]
            tailscale2[Tailscale bridge]
        end
        
        service_linux -->|bind| socket
    end
    
    subgraph "Clima Común (Cross-Platform)"
        runtime[(Runtime Lock: gnx-runtime.vhdx)]
        config{gnx.toml}
        
        Windows -.->|"mismo runtime"| runtime
        Linux -.->|"mismo runtime"| runtime
        
        runtime --> config
    end
    
    %% Conexiones entre host y WSL
    user -.->|no acceso directo| wsl
    cli -->|"protocolo binario allowlist (8 acciones)"| broker
    
    style Windows fill:#e1f5fe,stroke:#0277bd,stroke-width:2px
    style Linux fill:#f3e5f5,stroke:#8e24aa,stroke-width:2px
```

---

## 🏗️ Diagrama de Flujos de Instalación

### Flujo Windows

```mermaid
sequenceDiagram
    participant U as Usuario (mayas)
    participant Inst as Installer.exe
    participant Svc as GNXRuntime Service
    participant WSL as WSL2 + Podman
    
    U->>Inst: Ejecuta installer.exe
    Inst->>Svc: Crea cuenta gnx-runtime (SysAdmin)
    Inst->>WSL: Crea instancia Ubuntu 24.04 (vhdx pinado)
    Inst->>Svc: Instala quadlets en WSL
    Inst->>Svc: Configura bridge + firewall
    Inst-->>U: Entrega CLI al host Windows por protocolo binario
    
    Note over U,WSL: La distro WSL pertenece a gnx-runtime.<br/>El operador normal NO la ve en wsl --list.
```

### Flujo Linux Nativo

```mermaid
sequenceDiagram
    participant U as Usuario Operador
    participant App as gnx.appimage
    
    U->>App: Ejecuta gnx.appimage (sin root)
    App->>U: Instala binarios en ~/.local/bin
    App-->>U: Expone CLI y sockets de datos sensibles
    
    Note over U,App: No se crea usuario dedicado.<br/>No hay WSL ni Podman.<br/>Solo el servicio local con socket seguro.
```

---

## 🧩 Diagrama de Componentes del Runtime (WSL)

```mermaid
flowchart LR
    subgraph GNXRuntime["GNXRuntime — Ubuntu 24.04"]
        direction TB
        
        systemd[systemd]
        podman[Podman]
        
        quadlets{Quadlets}
        q1[gnx-access.service<br/>Tailscale + DNS]
        q2[gnx-dns.service<br/>dnsmasq + DoT]
        q3[gnx-compute.service<br/>bridge + firewall]
        q4[gnx-controller.service<br/>Caddy + CA .gnx]
        
        subnet[(bridge0: 172.25.0.0/16)]
        tailscale["tailscale0 (br-tlslch-*)"]
        host[eth0: acceso al host Windows]
    end
    
    subgraph "Aislamiento"
        direction TB
        no_shell[(No shell genérico<br/>no bash/zsh/sh)]
        no_exec[(No /bin/sh por defecto)]
        allowlist[Protocolo binario<br/>allowlist de 8 acciones]
        broker[Broker GNX]
        
        style no_shell fill:#ffeb3b,stroke:#f57f17
        style no_exec fill:#ffccbc,stroke:#c62828
        style allowlist fill:#c8e6c9,stroke:#2e7d32
    end
    
    systemd --> podman
    podman --> quadlets
    q1 -.-> tailscale
    q2 -.-> subnet
    q3 -.-> subnet
    
    %% Límites de seguridad
    GNXRuntime -.-> no_shell
    GNXRuntime -.-> no_exec
    broker -.-> allowlist
    
    style GNXRuntime fill:#e8f5e9,stroke:#1b5e20,stroke-width:2px
```

---

## 🔐 Diagrama de Límites de Confianza (Trust Boundaries)

```mermaid
flowchart TB
    subgraph Trusted["✅ De confianza (admin local)"]
        admin[Usuario administrador<br/>ej: maya]
        admin2[Acceso a /mnt/c/Windows/System32/]
    end
    
    subgraph Untrusted["⚠️ Riesgo accidental (operador normal)"]
        user[Operador normal<br/>cuenta de día a día]
        phishing[Sesión comprometida por phishing]
        malware[Malware en el host Windows]
    end
    
    subgraph GNXRuntimeIsolated["🔒 Aislado: gnx-runtime"]
        runtime_home[/home/gnx-runtime/]
        secrets{"Clave de enrollment<br/>Tailscale encriptada"}
        no_cwd[No puede montar /mnt/c]
        no_wsl[No ve wsl --list]
    end
    
    admin --> GNXRuntimeIsolated
    user -.->|no acceso directo| GNXRuntimeIsolated
    phishing --> GNXRuntimeIsolated
    malware --> GNXRuntimeIsolated
    
    boundary_note[El usuario gnx-runtime NO ve:<br/>/mnt/c/Windows/<br/>Claves de enrollment<br/>Estado de WSL en el host<br/>Datos sensibles de Proxmox/Tailscale]
    boundary_note -.-> runtime_home
```

---

## 📦 Diagrama de Artefactos de Distribución

```mermaid
flowchart LR
    subgraph Windows["🪟 Windows Installer"]
        direction TB
        installer[installer.exe]
        
        installer --> create_user[Crea usuario gnx-runtime]
        create_user --> create_wsl[Crea WSL2: Ubuntu 24.04 vhdx]
        create_wsl --> install_quadlets[Instala quadlets]
        install_quadlets --> config_bridge[Configura bridge + firewall]
        config_bridge --> deliver_cli[Entrega CLI al host Windows]
        
        runtime_lock[(runtime.lock.json<br/>SHA256 del VHD)]
        installer -.->|usa como referencia| runtime_lock
        
        style installer fill:#e3f2fd,stroke:#1565c0
    end
    
    subgraph Linux["🐧 Linux Native"]
        direction TB
        appimage[gnx.appimage]
        
        appimage --> install_binaries[Instala binarios en ~/.local/bin]
        install_binaries --> start_service["Inicia GNXService (socket)"]
        start_service --> cli_ready[CLI lista para uso]
        
        style appimage fill:#fce4ec,stroke:#c2185b
    end
    
    subgraph Common["🌐 Clima Común"]
        direction TB
        gnx_toml{gnx.toml}
        runtime_lock_common[(runtime.lock.json)]
        
        style gnx_toml fill:#fff9c4,stroke:#fbc02d
        style runtime_lock_common fill:#e8f5e9,stroke:#1b5e20
    end
    
    Windows -.-> common[Comparten: gnx.toml<br/>runtime.lock.json]
    Linux -.-> common
```

---

## 📋 Diagrama de Protocolo Binario del Broker (Windows)

```mermaid
stateDiagram-v2
    [*] --> Connected
    Connected --> RequestReceived: Recibe solicitud binaria
    RequestReceived --> Validated: Verifica allowlist (8 acciones)
    
    state Validated {
        [*] --> CheckSignature
        CheckSignature --> Denied: Firma inválida
        CheckSignature --> Authenticated
        Authenticated --> ExecuteAllowedAction
        ExecuteAllowedAction --> ResponseSent
        ResponseSent --> Connected
    }
    
    RequestReceived --> InvalidCommand: Acción no permitida
    InvalidCommand --> ResponseSent
    
    state "8 acciones allowlist" as actions {
        [*] --> tailscale_connect
        tailscale_connect --> tailscale_disconnect
        tailscale_disconnect --> dns_configure
        dns_configure --> dns_reset
        dns_reset --> compute_deploy
        compute_deploy --> compute_undeploy
        compute_undeploy --> compute_status
        compute_status --> health_report
        health_report --> config_get
        config_get --> health_check
        health_check --> status_report
    }
    
    ResponseSent --> Connected
    
    note right of Validated: Verifica firma RSA de la CLI, acción en allowlist (≤8) y parámetros dentro de límites.

    classDef connected fill:#e8f5e9,stroke:#1b5e20
    classDef validated fill:#fff3e0,stroke:#ef6c00
    class Connected connected
    class Validated validated
```

---

## 🔄 Diagrama del Ciclo de Vida del Servicio GNXRuntime

```mermaid
stateDiagram-v2
    [*] --> Installing: Ejecutar installer.exe
    
    Installing --> Running: Servicio iniciado (GNXRuntime)
    
    Running --> Stopping: Comando gnx stop o reinicio WSL
    Running --> Updating: Comando gnx update
    Updating --> Running
    
    Running --> Decommissioning: Comando gnx reset
    Decommissioning --> [*]
    
    note right of Running: El servicio corre bajo la cuenta dedicada gnx-runtime.<br/>La distro WSL pertenece a esa cuenta, no al usuario normal.

    classDef installing fill:#e3f2fd,stroke:#1565c0
    classDef running fill:#e8f5e9,stroke:#1b5e20
    classDef decommissioning fill:#ffebee,stroke:#c62828
    class Installing installing
    class Running running
    class Decommissioning decommissioning
```

---

## 📐 Diagrama de Red Virtual (Quadlet Compute)

```mermaid
flowchart LR
    subgraph "Bridge: bridge0"
        direction TB
        gateway[172.25.0.1]
        
        subnet[(172.25.0.0/16)]
        
        vm_pod1[Pod: VM 1<br/>eth0: 172.25.0.10]
        vm_pod2[Pod: VM 2<br/>eth0: 172.25.0.11]
        vm_pod3[Pod: VM 3<br/>eth0: 172.25.0.12]
    end
    
    subgraph "Tailscale Overlay"
        direction TB
        tlslch_1[br-tlschan-001<br/>10.42.0.10]
        tlslch_2[br-tlschan-002<br/>10.42.0.11]
    end
    
    subgraph "Host Windows"
        direction TB
        eth0["eth0: 192.168.x.x<br/>(IP del host)"]
    end
    
    bridge0{bridge0} --> gateway
    gateway -.-> subnet
    vm_pod1 -.-> bridge0
    vm_pod2 -.-> bridge0
    vm_pod3 -.-> bridge0
    
    bridge0 -.->|VLAN 42| tlslch_1
    bridge0 -.->|VLAN 42| tlslch_2
    
    eth0 -->|gateway NAT| subnet
    
    style bridge0 fill:#e8f5e9,stroke:#1b5e20,stroke-width:3px
```

---

## 📋 Resumen de Artefactos (Checklist)

| Archivo | Propósito | ¿Dónde se crea? |
|---------|-----------|-----------------|
| `installer.exe` | Instalar usuario + WSL + quadlets en Windows | Se empaqueta desde artefactos comunes |
| `gnx.appimage` | Binario de Linux (CLI + service socket) | Se empaqueta con los binarios del runtime |
| `runtime.lock.json` | Hash SHA256 del VHD del runtime | Pinado, no se genera dinámicamente |
| `config/gnx.toml` | Configuración declarativa | Editable por el usuario |
| `.gnx-runtime/vhdx` | Imagen del runtime WSL | Se pinza en Windows Installer |

---

## ✅ Validación de la Propuesta

### ¿Por qué esta propuesta es correcta?

1. **Menos código:** Los diagramas Mermaid son el contrato principal; el código solo implementa lo que los diagramas definen.

2. **Artefacto Windows = gestor de infraestructura:** Crea usuario, WSL, quadlets. No es "más invasivo" porque todo ocurre bajo una cuenta dedicada (`gnx-runtime`) y la distro WSL no aparece en el contexto del operador normal.

3. **Artefacto Linux = binario nativo:** Solo instala los binarios necesarios (CLI + service con socket). No crea usuarios ni máquinas virtuales extra.

4. **El runtime es compartido:** Un solo rootfs (Ubuntu 24.04) sirve tanto para Windows como para Linux nativo. La lógica de infraestructura es idéntica tras cruzar la frontera del broker.

5. **Podman nace dentro de WSL, no como VM extra:** `podman machine` no se usa. Podman corre directamente en el rootfs compartido, junto con systemd.

6. **Límites de confianza claros:** El usuario administrador (`maya`) tiene acceso total. Lo que se reduce es la superficie accidental del operador normal (phishing, malware, errores humanos).

---

## 📎 Apéndices

### A. Límite de 8 acciones en el broker

```
1. tailscale.connect / disconnect
2. dns.configure / reset
3. compute.deploy / undeploy
4. compute.status
5. health.report
6. config.get (NO secrets)
7. health.check
8. status.report
```

### B. Qué NO hace GNXRuntime

- No instala shell genérico (`bash`, `zsh`).
- No tiene `/bin/sh` por defecto.
- No permite exec arbitrario.
- No expone sockets TCP al host Windows (solo Unix sockets en WSL).
- No usa Podman Machine ni VMs anidadas extra.

### C. Qué SÍ hace GNXRuntime

- Instala `podman`, `systemd`, `openssl`, `curl`, `iproute2`.
- Inicia quadlets como unidades systemd.
- Monta un bridge para la red virtual (VMs + Tailscale).
- Excluye `/mnt/c/Windows/System32/winevt/eventlog` del mount de WSL.

---

## 📝 Notas sobre diagramas vs código

> "Menos código" significa que los diagramas Mermaid son el **contrato principal**. El código solo implementa lo que los diagramas definen. Los desarrolladores leen los diagramas como si fueran especificaciones formales.

Los 6 diagramas principales cubren:
1. Arquitectura global (host ↔ WSL)
2. Flujos de instalación (Windows vs Linux)
3. Componentes del runtime (quadlets, bridge, tailscale)
4. Límites de confianza (quién puede ver qué)
5. Protocolo binario del broker
6. Red virtual y overlay Tailscale

Esto elimina ambigüedades que normalmente se pierden en cientos de líneas de código con comentarios.

---

**Fin del documento.**
