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
| Linux runtime | Wide Linux `GNX-0.3.1`, systemd and Podman available where applicable |
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

- public GNX root certificate may be exported; private CA key stays protected;
- TLS validates chain, hostname and validity period without insecure overrides;
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
