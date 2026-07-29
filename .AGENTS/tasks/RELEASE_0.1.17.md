# Release 0.1.17 — installed MVP stabilization

## Objective

Deliver one bounded Windows-installed MVP that sizes itself from the host, recovers dependency installation, selects controller/member roles from online controller presence, starts PVE before publishing HTTPS and keeps every remote operation inside a documented argv/stdin/file contract.

## Allowed files

- Host-preflight inventory, profile persistence and their validators/tests.
- Service runtime profile, machine, Tailscale, topology, reconciler, remote transport and tests.
- Existing installer recovery modules and release wiring.
- Runtime payload sources only when an existing allowlisted operation requires correction.
- Version, installer identities, validators, release records and documentation.

## Prohibited changes

- No new crate, application, service, listener or port.
- No IPC or persisted-state schema change.
- No generic controller API, arbitrary repair command or caller-provided runtime argv.
- No free-form resource override.
- No `sh -c`, `bash -c`, remote shell redirection or command pipeline.
- No controller failover, multi-cluster-per-tailnet identity, HA or QDevice.

## Acceptance

- The observed 5864 MiB/4 CPU host calculates a bounded laboratory profile.
- `.wslconfig` and `podman machine init` consume the same selected profile.
- Zero online controllers promotes the node to controller.
- One or more online controllers creates a member; existing member count is irrelevant.
- Role state is committed after final Tailscale identity verification.
- PVE is ready before Serve is applied.
- Serve JSON reaches `tailscale serve set-raw` through bounded stdin.
- Remote-execution validators, Rust checks and WiX build pass on the designated platforms.
