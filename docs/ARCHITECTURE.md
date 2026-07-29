# Architecture

Quetzalcoatl is a Windows control plane around one managed Podman Machine. Setup
owns installation, upgrade and repair; the service owns reconciliation; CLI and
tray are bounded Named Pipe clients.

```text
QuetzalcoatlSetup.exe
  +-- gnx-bootstrap       closed host preflight, pinned dependencies, recovery
  `-- Quetzalcoatl.msi    gnx, tray, service, runtime and machine image
          `-- validate locked installation before StartServices

gnx / tray -- Named Pipe schema 2 --> gnx-service
                                         +-- Windows profile/state/secrets
                                         `-- Podman Machine bounded stdin
                                               +-- Tailscale
                                               `-- Proxmox
```

The workspace has exactly four packages:

| Path | Responsibility |
|---|---|
| `apps/gnx` | CLI, pipe client and native Win32 tray |
| `apps/gnx-service` | application lifecycle, domain rules and infrastructure |
| `apps/gnx-bootstrap` | host preflight, dependencies and reboot recovery |
| `crates/gnx-contracts` | shared IPC, status, host profile and schema constants |

`crates/` is the standard Rust workspace location for shared libraries; it is not
vendor storage. Application sequencing belongs in `application/`, rules and closed
types in `domain/`, and operating-system/process adapters in `infrastructure/`.

The service lifecycle is: host profile and WSL, pinned Podman Machine, Fedora/KVM,
locked payload, protected configuration, Tailscale identity and role, local PVE
readiness, fixed HTTPS Serve route, controller creation or bounded member join, then
atomic `READY`. Zero online controllers selects controller; one or more selects
member. Existing members do not affect the choice. Persisted valid identity wins on
restart.

The native tray intentionally adds no localhost UI, listener, port or Tauri runtime.
