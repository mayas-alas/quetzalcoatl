# Quetzalcoatl architecture — 0.1.17

Quetzalcoatl remains a four-package Rust workspace plus a versioned Fedora payload and a WiX installer.

```text
crates/
├─ gnx-cli/              user-facing CLI
├─ gnx-protocol/         Named Pipe schema and shared response models
├─ gnx-service/          Windows service and runtime reconciliation
└─ gnx-host-preflight/   installer host checks and dependency staging
```

## CLI boundary

`gnx-cli` owns parsing, local rendering and Named Pipe access. `-v` and `--version` terminate locally through `CARGO_PKG_VERSION`; `status`, `configure` and `restart` preserve their existing behavior.

## Installer boundary

Burn prepares Windows features and invokes typed host-preflight helper modes. WSL and Podman arrive as ancillary payloads, are validated and copied into the GNX-owned stable cache, and only then are executed through `msiexec`. The helper accepts no arbitrary payload path or version.

## Service boundary

```text
gnx-service/src/
├─ main.rs       process composition root
├─ service/      service startup ownership
├─ ipc/          local Named Pipe server and authorization
├─ secrets/      protected configuration persistence
├─ state/        persisted runtime state and validation
└─ runtime/      reconciliation and infrastructure adapters
   ├─ remote/    bounded machine transport and typed Fedora-agent operations
   └─ cluster/   bounded member join coordination
```

IPC consumes shared protocol commands and never calls Podman, Tailscale or Proxmox implementation modules directly. Runtime startup passes through `RuntimeControl` and the reconciler.

## Remote-execution boundary

Every operation crossing into Fedora or a container follows three separate channels:

```text
argv   → closed operation identity and validated scalar arguments
stdin  → bounded variable data, JSON, scripts and secrets
files  → durable GNX-owned state only
```

The transport enforces maximum input/output, timeout and cancellation. `RuntimeOperation` closes the Fedora-agent command set. Direct machine/container operations are allowed only with fixed argv and the policy in `REMOTE_EXECUTION.md`. Shell strings, redirection and generic execution are outside the architecture.

## Topology boundary

New-node selection depends only on online controller presence. No controller promotes the node; one or more controllers creates a member. Members are not counted. Persisted upgrades preserve their role and controller identity.

## Serve boundary

Tailscale connectivity starts before PVE, but HTTPS Serve does not. The reconciler waits for the local PVE backend, serializes the fixed Serve route in Rust and sends it to the Tailscale CLI through stdin inside `gnx-tailscaled`.

## Member boundary

The member keeps the typed, idempotent join operation. Rust coordinates prepare, authorize, join, verify and confirm phases; Fedora executes only allowlisted runtime-agent operations. Confirmation observes PVE cluster state and does not open a new GNX endpoint.

## Versioned contracts

- Product version: `0.1.17`
- Protocol schema: `2`
- Persisted-state schema: `2`
- Runtime generation: `proxmox-cluster-v2`
- Runtime payload: `5`

See `TARGET_0.2.md` for boundaries deliberately deferred until the installed MVP is proven.
