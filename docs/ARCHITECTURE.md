# Quetzalcoatl architecture — 0.1.13

## Product boundary

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
        +--> exact runtime payload v4
        +--> on-demand typed gnx-runtime-agent
        |
        v
Podman pod
  +--> Proxmox VE
  +--> Tailscale
        |
        v
persistent role resolution
  +--> controller: create or verify cluster
  +--> member: resume and join pinned controller
        |
        v
READY
```

## Workspace

The workspace remains limited to:

```text
crates/
├─ gnx-protocol
├─ gnx-cli
├─ host-preflight
└─ gnx-service
```

No new crate, service, listener or remote API is introduced.

## CLI boundary

The installed `gnx.exe` exposes three commands: `status`, `configure` and `restart`. `status --json` returns protocol v2 JSON; human status exposes the same overall, stage, role, controller, component and cluster fields. The CLI rejects responses with a different schema version. WiX installs one keyed CLI binary, registers `[INSTALLFOLDER]` in the system PATH and verifies its hash during administrative extraction.

## Windows runtime modules

```text
gnx-service/src/runtime/
├─ mod.rs                 facade and fixed release constants
├─ reconciler.rs          convergence sequence
├─ error.rs               runtime error classification
├─ model.rs               internal data contracts
├─ status.rs              observable status transitions
├─ host.rs                service identity and WSL configuration
├─ machine.rs             Podman Machine lifecycle and devices
├─ payload.rs             manifest and atomic payload application
├─ tailscale.rs           enrollment, identity, Serve and health
├─ proxmox.rs             PVE startup, identity and cluster operations
├─ topology.rs            persistent role and member join
├─ remote/
│  ├─ operation.rs        closed Fedora operation enum
│  ├─ client.rs           typed agent client
│  ├─ transport.rs        Podman Machine process transport
│  ├─ limits.rs           input, output and timeout contracts
│  └─ mod.rs
└─ tests.rs
```

`mod.rs` starts the runtime and maps terminal failure. `reconciler.rs` owns the established 0.1.11 sequence; stage names, state schema and controller/member behavior remain unchanged.

## Fedora payload

`runtime/payload` is the only source payload. `manifest.json`, the physical file set and Rust `PAYLOAD_FILES` form one exact 12-file contract. Payload version 4 changes the runtime agent hardening and PVE credential helper but does not change machine generation.

## Execution boundary

After atomic payload application and handshake, sensitive PVE and Tailscale operations are selected through `RuntimeOperation` and dispatched by `gnx-runtime-agent`. The agent executes fixed payload paths only. It has no listener, daemon mode or generic exec branch.

Bootstrap and health probes that need multiple shell statements use fixed programs sent to `sh -s` through stdin. Dynamic values use argv or stdin. Managed runtime code contains no `sh -c` or `bash -c` command-string execution.

## Managed generation and persistence

The managed generation remains `proxmox-cluster-v2`. Existing compatible machines receive payload v4 without forced recreation. An incompatible generation preserves only managed Tailscale state, recreates the machine, resets the cluster checkpoint and reapplies the runtime.

Protected state continues to store the node identity, role, controller identity, tailnet and join checkpoint. A persisted controller is verified rather than recreated; a member resumes against its pinned controller.
