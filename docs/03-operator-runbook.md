# GNX 0.3.1 operator runbook

Status: normative operational guide  
Audience: operator, support, audit  
Last reviewed: 2026-09-19  
Replaces: `access.md`, `control.md`, `compute.md`, operational parts of setup docs

This runbook describes how GNX should be operated and diagnosed. It does not override product requirements (`01-product.md`) or acceptance gates (`05-acceptance.md`).

## Safe operating rules

- Use `gnx doctor` before mutation.
- Use `gnx plan` to inspect intended changes; it must not mutate.
- Treat `ACTION_REQUIRED` as a bounded next step, not success.
- Never provide secrets through argv, environment, logs, screenshots or evidence.
- Do not mount or modify `legacy`.
- Do not call a generated file, running process, web console or container `READY` without the required live probe.
- Preserve last valid state and private identity during recovery.

## Clean-host gate

Before a clean install or acceptance run, verify and record:

| Gate | Required observation |
| --- | --- |
| Host | supported OS/architecture and elevated rights only where required |
| Linux runtime | Native Linux or Wide Linux `GNX-0.3.1`, systemd, cgroup v2, Podman **6+**, and the installed Quadlet system generator |
| Devices | `/dev/kvm`, `/dev/net/tun` or other required devices only for the selected release |
| Network | private transport prerequisites, no unintended public bindings |
| State | no conflicting `C:\Program Files\GNX`, `C:\ProgramData\GNX`, `GNX`, `gnx-node` adoption or stale setup journal unless explicitly recovering |
| Release | authenticated manifest and exact artifact/rootfs digests |

A missing prerequisite returns `FAILED <GATE>` or `ACTION_REQUIRED <GATE>` with sanitized next action.

## Normal command sequence

```text
gnx doctor
gnx plan
gnx apply
gnx status
```

Expected behavior:

- `doctor` explains prerequisites and release issues before mutation.
- `plan` compares desired intent with observed state.
- `apply` performs transactional reconciliation and verification.
- `status` reports live capability health.

Each command emits one JSON document. Use shell redirection only for non-secret output.

## Access operation

Access establishes private identity, authorized transport and `.gnx` DNS. The operator may need to provide enrollment material through a no-echo prompt only after `ACTION_REQUIRED`.

Required operational facts:

- node identity persists across reapply and reboot;
- `.gnx` answers are authoritative for declared names only;
- names outside `.gnx` are refused rather than recursively resolved;
- split DNS is used; GNX DNS is not configured as a global resolver;
- private transport policy, DNS reachability and TLS trust are separate gates.

Historical Android/Tailscale evidence proved a useful pattern: human-entered one-off key, temporary `0600` file in tmpfs, no secret in argv, and explicit SaaS split-DNS fields. That evidence does not replace current G2 acceptance.

## Control operation

Control publishes explicit HTTPS names and routes each one to a declared upstream.

Required operational facts:

- the GNX public root certificate is exported by the broker and automatically installed in the interactive operator's Windows user trust store; the private CA key stays protected;
- TLS validates chain, hostname and validity period without insecure overrides;
- browser restart is only a cache refresh after trust installation, not a substitute for certificate validation;
- each route is explicit; no wildcard fallback;
- direct Compute ports and local Control interfaces are not remotely reachable;
- optional routes may fail without degrading required capabilities.

Historical control-plane notes retained as rules: keep protected state under ProgramData/Wide Linux root-only locations, preserve existing identity on reprepare, renew certificates before expiry, and do not log tokens or request bodies.

## Compute operation

Compute is the persistent authenticated service behind Control.

Required operational facts:

- service artifact matches release identity;
- persistent storage is available;
- health probe is authenticated and confirms the expected service identity;
- Control receives only a non-secret upstream handoff;
- credentials are not printed, logged, copied to clipboard or stored in evidence.

The old Proxmox/Dockur deployment is historical evidence of how to run a first Compute service. It is not a product promise of VM/LXC provisioning for 0.3.1.

## Quadlet lifecycle and an existing LXC

This implements BR-K01 and BR-P06 without introducing a new capability, naming tier, workload scheduler, or LXC provisioner. Existing GNX unit/container names and persistent paths are unchanged. LXC creation and host boot policy remain operator responsibilities.

The Linux adapter writes managed `.container` files under `/etc/containers/systemd/`. Quadlet generates the corresponding `.service` files. `WantedBy=multi-user.target` supplies boot startup; `Restart=on-failure` with bounded restart attempts supplies crash recovery without hiding a permanently broken service. Do **not** run `systemctl enable` on generated services. GNX reloads systemd, verifies the service's `SourcePath`, and starts/restarts only its named units. Existing capability probes still decide health; generated files alone are not acceptance.

Before running in an already-created LXC:

1. Confirm its identity on the virtualization host and enable that guest's boot startup explicitly. Guest Quadlets cannot start a stopped LXC.
2. Inside the guest, verify systemd is PID 1, cgroup v2 is delegated/writable, and `podman version --format '{{.Client.Version}}'` reports major version 6 or newer. Install Podman and its Quadlet generator from an approved distribution source; do not bypass the version check.
3. Verify nested OCI containers are supported by the host's approved LXC configuration. Supply only the devices required by the signed release (the current Compute adapter checks KVM, FUSE and TUN). Do not disable AppArmor, grant blanket privileges, or change unrelated guests merely to silence a gate.
4. Install the verified GNX release, preserve protected state, then run `gnx doctor`, `gnx plan`, `gnx apply`, and `gnx status` **inside that Linux runtime**. The Windows checkout or a browser response is not guest acceptance.
5. Inspect each generated service's `SourcePath` and live health. In an approved maintenance window, verify guest restart and host reboot restore the same identity, storage and authenticated service health without reapplying.

### Existing handwritten services

`QUADLET_MIGRATION_REQUIRED` means an existing `.service` shadows the intended generated unit. GNX refuses **before reconciliation**: it does not stop, disable, delete or adopt that file. `RUNTIME_UNIT_CONFLICT` similarly protects unowned or symlinked Quadlet files. Back up the exact unit configuration, last-valid state and protected storage; review unit ownership and dependencies; schedule an explicit maintenance migration. Only the operator may retire the matching old unit after review. Preserve its backup for rollback and never remove persistent volumes, CA keys, credentials or network identity. Then reload systemd and rerun the normal command sequence. If migration is not approved, keep the previous release and running services.

`PODMAN_6_REQUIRED`, `QUADLET_GENERATOR_REQUIRED` and `QUADLET_GENERATION_FAILED` remain failed gates, not successful deployment. Host acceptance for nested LXC, generation on Podman 6+, crash recovery and reboot is still required.

## Troubleshooting states

| Symptom | Interpret as | Do next |
| --- | --- | --- |
| `ACTION_REQUIRED` with secret kind | protected input needed | collect through approved no-echo prompt only |
| `ACTION_REQUIRED SETUP_PROVISIONED` | setup stage finished but runtime not verified | continue to bootstrap/health gate; do not claim READY |
| HTTP 200 from noVNC/web UI | console endpoint is reachable | inspect guest/runtime gate before any readiness claim |
| container running, QEMU absent | host container started but guest not booted | keep as BLOCKED unless guest evidence appears |
| process/service active | lifecycle intermediate fact | run capability health probe |
| missing JSON or multiple stdout documents | contract failure | fix command rendering before interpreting payload |

## Recovery

- Stop only the exact GNX service/container/unit being recovered.
- Preserve private identity, CA material and Compute data unless a documented destructive recovery is approved.
- If setup created a journal/snapshot, inspect it before retry. Blind deletion can hide partial state.
- Rollback may remove only targets it can prove belong to the exact versioned GNX install.
- Existing legacy directories are not adopted or modified to make a gate pass.

## Evidence hygiene

Record versions, commit, artifact digests, platform versions, exact gates and sanitized results. Do not record credentials, private keys, tokens, authorization headers, update URLs, cookies, screenshots with secrets, or full logs that may contain those values.
