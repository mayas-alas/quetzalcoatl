# GNX 0.3.1 product contract

Status: normative  
Audience: product, engineering, audit  
Last reviewed: 2026-09-19  
Replaces: `business-requirements.md`, product parts of `documentation-audit.md`

GNX is private infrastructure behind one small, verifiable command contract. It reconciles one node around three business capabilities:

- **Access**: authorized private reachability and authoritative `.gnx` DNS.
- **Control**: explicit HTTPS names, TLS and routing to declared upstreams.
- **Compute**: the persistent service behind Control with its own authentication and storage.

Concrete products such as WSL, Podman, CoreDNS, Caddy, Tailscale, NetBird or Dockur are release/adaptor choices. They are not public capability names.

## Required outcome

An authorized remote client can resolve and open `https://compute.gnx` over a private connection. TLS is trusted only after deliberate installation of the GNX public root. Compute still performs its own authentication. Reapply, service restart and reboot preserve private identity, last-valid configuration and Compute storage.

## Public command contract

```text
gnx doctor    # validate host/release prerequisites and explain corrective action
gnx plan      # compare intent with observed state; never mutate
gnx apply     # reconcile a validated candidate and verify the result
gnx status    # report observed capability health
```

Stdout is one machine-readable JSON document. Human diagnostics go to stderr.

| State | Exit | Meaning |
| --- | ---: | --- |
| `READY` | 0 | required live checks passed |
| `FAILED` | 1 | operation failed and did not produce a valid candidate |
| `ACTION_REQUIRED` | 2 | bounded next action is needed; this is not success |

Generated files, compiled binaries, active processes, HTTP 200 from a console and container startup are evidence, but they do not by themselves justify `READY`.

## Product vocabulary

| Term | Meaning |
| --- | --- |
| Authorized client | client admitted by the selected private transport policy |
| GNX node | one instance of Access, Control and Compute in a shared Linux runtime |
| Intent | portable, non-secret operator input, normally `gnx.toml` |
| Release | authenticated immutable implementation choices and artifacts supplied by GNX |
| Observed state | facts read from the running host and services |
| Candidate | validated intent plus one release, staged but not yet promoted |
| Last valid | most recent candidate whose required live checks passed |
| Required route | `compute.gnx`; failure affects Control acceptance |
| Optional route | explicit route to software whose lifecycle GNX does not own |

## Requirements

### Access

- **BR-A01 Private identity.** The node has one persistent private-network identity. Reapply and reboot preserve it.
- **BR-A02 Private transport.** Only authorized clients can reach GNX service entrypoints over the private network.
- **BR-A03 Name reachability.** The normal client resolver resolves `.gnx`. GNX DNS is authoritative only for that zone and is not a general resolver.
- **BR-A04 Boundary.** Access owns identity, transport and private DNS reachability; it does not own HTTPS routing or Compute lifecycle.

### Control

- **BR-C01 Stable entrypoint.** `https://compute.gnx` is the stable client contract.
- **BR-C02 Verified TLS.** Control terminates HTTPS with a hostname-valid certificate anchored in the deliberately trusted GNX public root.
- **BR-C03 Explicit routing.** Every published hostname maps explicitly to one upstream. There is no wildcard fallback.
- **BR-C04 Isolation.** Compute ports and local Control interfaces are not remotely exposed.
- **BR-C05 Optional routes.** Failure of an optional externally managed route does not degrade required Access, Control or Compute health.

### Compute

- **BR-K01 Persistent service.** Compute uses persistent storage and reports real authenticated health.
- **BR-K02 Independent authentication.** Reaching Control never bypasses Compute credentials.
- **BR-K03 Boundary.** Compute does not own client identity, DNS or public routing.

### Product lifecycle

- **BR-P01 Declarative intent.** `gnx.toml` describes desired node intent only. It does not select image repositories, digests, providers, internal paths or secrets.
- **BR-P02 Immutable release.** One internal release definition pins every component and default.
- **BR-P03 Four use cases.** Public operations are exactly `doctor`, `plan`, `apply` and `status`.
- **BR-P04 Safe reconciliation.** `plan` observes without mutation. Unchanged `apply` is idempotent. Invalid candidates never replace last valid.
- **BR-P05 Honest results.** JSON/exit semantics are authoritative; missing runtime checks cannot be converted into success.
- **BR-P06 Recovery.** Linux reboot and the documented Windows/WSL recovery path restore capabilities without recreating identity or storage.
- **BR-P07 Platform parity.** Windows `gnx.exe` and Linux `gnx` expose the same operations, JSON schema and exit semantics.
- **BR-P08 Single orchestration core.** Windows is a typed bridge to the Linux runtime; it does not duplicate reconciliation logic.
- **BR-P09 Secret boundary.** Enrollment credentials, passwords, cookies, authorization headers and private keys are runtime state, never intent, argv, environment, logs or evidence.

### Windows host

- **BR-W01 Dedicated identity.** Setup creates/reconciles local standard account `gnx-runtime`, grants service-logon and denies interactive, remote-interactive and network logon.
- **BR-W02 Managed service.** Setup creates/updates service `GNXRuntime` under that account and verifies readiness before success.
- **BR-W03 Private host state.** `C:\ProgramData\GNX-0.3.1` is protected for SYSTEM, Administrators and the runtime account.
- **BR-W04 Closed broker.** `gnx.exe` talks to `GNXRuntime` through a local named pipe with bounded frames and only the four allowed operations.
- **BR-W05 Separate secret channel.** Secrets are requested only after `ACTION_REQUIRED`, read without echo, carried in a dedicated bounded frame and delivered to Linux through stdin.
- **BR-W06 Product-owned Linux artifact.** Windows release contains the GNX Linux binary built by the same release pipeline and pinned by manifest.
- **BR-W07 Isolated WSL runtime.** `GNXRuntime` owns fixed distro `GNX-0.3.1`; automount and Windows interop are disabled.
- **BR-W08 Fixed Linux execution.** Service invokes only `/usr/local/bin/gnx` with fixed config path and allowlisted argv.
- **BR-W09 Public export only.** Windows may receive the public GNX CA certificate and sanitized JSON results; private keys never cross back.
- **BR-W10 Explicit trust boundary.** The design protects the normal operator session from accidental access. SYSTEM and local Administrators remain trusted.

## Release constraints and non-goals

For 0.3.1, Access, Control and Compute run in one shared Linux runtime. CoreDNS is the fixed DNS adapter. Windows uses distro `GNX-0.3.1`; existing `GNX` or `gnx-node` distros are legacy/conflicting state and are never adopted by setup.

Out of scope: provider/plugin frameworks, workload catalogs, VM/LXC provisioning, schedulers, HA, automatic upgrades, tray applications and commercial installers. GNX does not claim isolation from Linux root, Windows SYSTEM/local Administrators or a compromised privileged runtime.

## Automation and agent rule

Agent coordination may help development, evidence review or documentation, but it is **outside** the product runtime. No agent harness, subagent protocol, scheduler or orchestration worker is implemented inside `gnx`, `gnx-service`, setup, release artifacts or installers. Public interfaces keep GNX names; legal attribution remains in licenses/SBOM/evidence.
