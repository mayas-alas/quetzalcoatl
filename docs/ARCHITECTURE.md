# Quetzalcoatl architecture

## Product boundary

Quetzalcoatl prepares a Windows 11 host as a Proxmox VE cluster node. Its runtime owns only the path required to reach a healthy, joined and quorate cluster state.

```text
Windows service + CLI
        |
        v
WSL2 configuration
        |
        v
Dedicated Fedora Podman Machine
        |
        +--> KVM, TUN and FUSE validation
        |
        v
Podman pod
  +--> Proxmox VE
  +--> Tailscale
        |
        v
persistent role resolution
  +--> controller: create or verify cluster
  +--> member: discover and join controller
        |
        v
READY
```

## Windows components

- `gnx-service`: convergence authority running under the dedicated Windows service identity.
- `gnx`: local CLI for status, protected configuration and service restart.
- Named pipe: authenticated local protocol between CLI and service.
- DPAPI: protects the tailnet, Tailscale auth key and PVE root password.
- `state.json`: stores only schema version, node identity, role, controller identity and cluster checkpoint.

## Fedora runtime

The payload contains exactly 11 files:

- six executable scripts for PVE and Tailscale operations;
- one Tailscale Serve configuration;
- three Quadlet definitions;
- one systemd enrollment unit.

`manifest.json` and the Rust `PAYLOAD_FILES` array form the exact payload allowlist. Every file is LF-only, SHA-256 locked, written atomically and installed with a fixed mode.

## Runtime state machine

Common stages:

```text
SERVICE_READY
RUNTIME_IDENTITY
WSL_PREPARING
MACHINE_PREPARING
MACHINE_NETWORK_PREPARING
MACHINE_READY
KVM_CHECKING
KVM_READY
PAYLOAD_APPLYING
PROXMOX_STARTING
POD_NETWORK_PREPARING
PROXMOX_CHECKING
PROXMOX_READY
CONFIGURATION_WAITING
PVE_CREDENTIAL_APPLYING
TAILSCALE_ENROLLING
TAILSCALE_CHECKING
ROLE_DISCOVERING
ROLE_RESOLVED
TAILSCALE_SERVE_CHECKING
TAILSCALE_READY
```

Controller completion:

```text
CONTROLLER_CLUSTER_PRECHECK
CONTROLLER_CLUSTER_CREATING
CONTROLLER_CLUSTER_READY
READY
```

Member completion:

```text
MEMBER_JOINING
READY
```

`READY` means the local PVE runtime is healthy and cluster membership is quorate.

## Persistence

Schema 2 contains only the cluster contract. A schema 1 record is normalized once by selecting the current identity, role, controller and join fields; supplementary fields are not persisted. A resumed controller verifies its existing cluster before returning to `READY`.

## Security invariants

- Secrets never enter process arguments, persisted state, runtime status or logs.
- Secret files under `/run/gnx` are root-owned, tightly permissioned and removed by traps.
- PVE access is available through Tailscale Serve; no PVE listener is published on Windows.
- Node role and controller identity cannot silently drift after persistence.
