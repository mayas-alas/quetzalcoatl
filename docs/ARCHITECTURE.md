# Quetzalcoatl architecture — 0.1.14

Quetzalcoatl remains a four-package Rust workspace plus a versioned Fedora payload and a WiX installer.

```text
crates/
├─ gnx-cli/              user-facing CLI
├─ gnx-protocol/         Named Pipe schema and shared response models
├─ gnx-service/          Windows service and runtime reconciliation
└─ gnx-host-preflight/   installer host checks
```

## CLI boundary

`gnx-cli` is organized by commands. `main.rs` only parses, dispatches and maps process exit behavior. `client.rs` owns Named Pipe transport and protocol-version checks. Output rendering and interactive configuration input are isolated from transport.

## Service boundary

```text
gnx-service/src/
├─ main.rs       process composition root
├─ service/      service startup ownership
├─ ipc/          local Named Pipe server and authorization
├─ secrets/      protected configuration persistence
├─ state/        persisted runtime state and validation
└─ runtime/      reconciliation and infrastructure adapters
```

The IPC layer can read status and store validated configuration, but it cannot invoke Podman, Tailscale or Proxmox modules directly. `RuntimeControl` owns runtime startup and delegates to the existing reconciler.

## Compatibility preserved

- Protocol schema: 2
- Named Pipe: `\\.\pipe\Quetzalcoatl`
- CLI: `status`, `configure`, `restart`
- Runtime generation: `proxmox-cluster-v2`
- Payload contract: version 4
- MSI and Burn upgrade families: unchanged
