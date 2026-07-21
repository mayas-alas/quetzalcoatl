# Scope and definition of done

## Functional MVP

The same `QuetzalcoatlSetup.exe` is installed sequentially on three compatible, clean Windows 11 consumer hosts:

```text
Host 1 -> controller
Host 2 -> member
Host 3 -> member
```

The result is one quorate, three-node Proxmox cluster whose Corosync `ring0_addr` values use direct Tailscale paths. Only the controller executes OpenTofu and owns the single Garage and Forgejo instances.

Roles, cluster membership, services, and secret handling survive individual node reboots. No Proxmox port is published on Windows.

## Required network gate

Before cluster acceptance, all three host pairs must demonstrate:

- direct Tailscale path, never DERP;
- zero packet loss;
- RTT below 5 ms;
- stable names and synchronized clocks;
- functional MTU;
- TCP 22 and 8006 between Proxmox guests;
- UDP 5405-5412 for Corosync;
- no Proxmox listener exposed on Windows.

GitHub-hosted Dockur runs cannot satisfy this gate. They prove compatibility and installer/runtime behavior only.

## Member contract

- Role is selected exactly once and persisted.
- Discovery excludes self, expired peers, service sidecars, and peers without the exact product tag.
- Exactly one controller must be identifiable.
- Join is secure, idempotent, resumable, and pinned to the original controller.
- OpenTofu, Garage, Forgejo, controller workspace, and controller credentials are `not_applicable` on members.
- Member-specific failures use stable operational error codes.

The implementation must not hard-code ordinal member identities. This cycle accepts exactly two members; support for a fourth node or arbitrary N-node clusters is deferred until after the three-node gate is verified.

## Pilotable MVP

After functional acceptance, release readiness additionally requires signed artifacts, CPU/RAM/disk preflight, a redacted diagnostics bundle, an operator guide, and verified clean install, resume, upgrade, repair/reinstall, uninstall, and recovery behavior.

## Exclusions

Do not implement tray UI, Headscale, Forgejo Runner, HA, controller election or promotion, concurrent first installs, multicloud, multiple runtimes, S3 state, gRPC, advanced command-center UI, a general migration framework, destructive purge, or remote telemetry.
