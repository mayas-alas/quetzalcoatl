# Business requirements — GNX 0.3.1

This document is the product contract for the clean 0.3.1 PoC. Architecture and
implementation choices must trace back to these requirements. A component name
is not a business capability.

`Must` requirements are required for PoC acceptance. `May` describes permitted
behavior, not an acceptance requirement. When an implementation constraint and a
business requirement conflict, the implementation changes or the requirement is
explicitly revised; the constraint does not silently redefine the product.

## Product language

| Term | Meaning in 0.3.1 |
| --- | --- |
| Authorized client | a remote client admitted by the selected private transport policy; host administration alone does not make it an authorized product client |
| GNX node | one instance of the three capabilities in a shared Linux runtime |
| Intent | portable, non-secret operator input describing the desired node |
| Release | authenticated, immutable implementation choices and artifacts supplied by GNX |
| Observed state | facts read from the running host and services through ports |
| Candidate | validated intent plus one release, staged but not yet promoted |
| Last valid | the most recent candidate whose required live checks passed |
| Required route | `compute.gnx`, whose failure affects Control acceptance |
| Optional route | an explicit route to software whose lifecycle GNX does not own |

Access, Control and Compute are always capitalized when they mean GNX business
capabilities. Concrete component names belong to release or adapter vocabulary.

## Required outcome

An authorized remote client can resolve and open `https://compute.gnx` over a
private connection. TLS is trusted only after deliberate installation of the GNX
public root, Compute performs its own authentication, and the complete service
recovers without recreating identity or persistent storage.

## Capability requirements

### Access

- **BR-A01 — Private identity.** The node has one persistent private-network
  identity. Reapply and reboot preserve it.
- **BR-A02 — Private transport.** Only authorized clients can reach GNX service
  entrypoints over the private network.
- **BR-A03 — Name reachability.** The normal client resolver resolves the `.gnx`
  private zone. GNX DNS is authoritative only for that zone and does not become a
  general resolver.
- **BR-A04 — Boundary.** Access owns identity, transport and private DNS
  reachability. It does not own HTTPS routing or Compute lifecycle.

### Control

- **BR-C01 — Stable entrypoint.** `https://compute.gnx` is the stable client
  contract. Internal container and provider names are not public contracts.
- **BR-C02 — Verified TLS.** Control terminates HTTPS with a certificate valid for
  the requested hostname and anchored in the intentionally trusted GNX root.
- **BR-C03 — Explicit routing.** Every published hostname maps explicitly to one
  upstream. There is no wildcard fallback to another service.
- **BR-C04 — Isolation.** Compute ports and local Control interfaces are not
  remotely exposed.
- **BR-C05 — Optional routes.** Failure of an optional route to an externally
  managed application does not degrade Access, Control or Compute.

### Compute

- **BR-K01 — Persistent service.** Compute uses persistent storage and reports
  real, authenticated health.
- **BR-K02 — Independent authentication.** Reaching Control does not bypass
  Compute credentials.
- **BR-K03 — Boundary.** Compute does not own client identity, DNS or public
  routing.

## Product and lifecycle requirements

- **BR-P01 — Declarative intent.** `gnx.toml` describes node intent only. It does
  not select image repositories, digests, provider families or internal service
  addresses.
- **BR-P02 — Immutable release.** One internal release definition pins every
  component and supplies implementation defaults.
- **BR-P03 — Four use cases.** The public command contract is `gnx plan`,
  `gnx apply`, `gnx status` and `gnx doctor`.
- **BR-P04 — Safe reconciliation.** `plan` observes without mutation. An unchanged
  `apply` is idempotent. Invalid candidate intent never replaces the last valid
  state.
- **BR-P05 — Honest results.** Stdout is JSON. `READY` exits `0`, `FAILED` exits
  `1`, and `ACTION_REQUIRED` exits `2`. Compilation or generated configuration is
  not runtime acceptance.
- **BR-P06 — Recovery.** Linux reboot and the documented Windows/WSL recovery path
  restore all three capabilities without recreating identity or storage.
- **BR-P07 — Platform parity.** Windows `gnx.exe` and Linux `gnx` expose the same
  operations, JSON schema and exit semantics.
- **BR-P08 — Single orchestration core.** Windows is a typed bridge to the Linux
  runtime in the dedicated `GNX` WSL distribution. It does not duplicate business
  or reconciliation logic.
- **BR-P09 — Secret boundary.** Enrollment credentials, passwords, cookies,
  authorization headers and private keys are runtime state, never node intent,
  command-line values, logs or evidence.

## Windows host requirements

- **BR-W01 — Dedicated identity.** Installation creates or reconciles the local
  standard account `.\gnx-runtime`. It has service-logon permission and explicit
  denial of interactive, remote-interactive and network logon. Its random
  credential is transferred only to SCM and zeroized after registration.
- **BR-W02 — Managed service.** Installation creates or updates the automatic
  `GNXRuntime` Windows service under `.\gnx-runtime`, configures bounded restart
  recovery and verifies service readiness before reporting success.
- **BR-W03 — Private host state.** `C:\ProgramData\GNX` has protected ACLs limited
  to SYSTEM, Administrators and `.\gnx-runtime`. The normal operator receives no
  direct access to the WSL disk, bootstrap material or runtime secrets.
- **BR-W04 — Closed broker.** `gnx.exe` reaches `GNXRuntime` through a local-only
  named pipe authorized for SYSTEM, Administrators and the installing operator
  SID. The protocol has bounded frames and opcodes only for `plan`, `apply`,
  `status` and `doctor`; it cannot carry an arbitrary command or path.
- **BR-W05 — Separate secret channel.** A secret is requested only after the Linux
  runtime returns `ACTION_REQUIRED`. It is read without echo, carried in a
  dedicated bounded frame and delivered to Linux through stdin. It never appears
  in `gnx.toml`, argv, environment, Windows files, logs or broker responses.
- **BR-W06 — Product-owned Linux artifact.** The Windows release contains the GNX
  Linux binary built by the same release pipeline. An authenticated release
  manifest pins its digest together with the service, CLI, runtime assets and WSL
  rootfs. Installation rejects any mismatch before execution.
- **BR-W07 — Isolated WSL runtime.** `GNXRuntime` imports and owns the fixed `GNX`
  distribution. Automount and Windows interop are disabled. The verified Linux
  bundle crosses the boundary through stdin, is installed root-owned, and its
  bootstrap copy is removed after success.
- **BR-W08 — Fixed Linux execution.** The service invokes only the product binary
  at `/usr/local/bin/gnx`, with fixed configuration path and argv derived from an
  allowlisted opcode. Linux remains the sole owner of use-case orchestration.
- **BR-W09 — Public export only.** Windows may receive the public GNX CA
  certificate and sanitized JSON results. Private keys and persistent service
  secrets never cross back from Linux.
- **BR-W10 — Explicit trust boundary.** The design protects the operator's normal
  session from accidental access and limits product surfaces. SYSTEM and local
  Administrators remain trusted and are not claimed as hostile-administrator
  boundaries.

## Release constraints for 0.3.1

- Access, Control and Compute run in one shared Linux runtime.
- CoreDNS is the fixed 0.3.1 DNS adapter. Its name and configuration remain under
  `adapter` and `runtime/access`; business and application modules depend only on
  the DNS/Access port.
- Windows uses a dedicated WSL distribution named `GNX`.
- Provider/plugin frameworks, workload catalogs, VM/LXC provisioning, schedulers,
  HA, automatic upgrades, tray applications and commercial installers are out of
  scope.

## Acceptance traceability

| Requirements | PoC gate |
| --- | --- |
| BR-K01, BR-K02 | G1 — Compute |
| BR-A01, BR-A02, BR-A03 | G2 — Access |
| BR-C01, BR-C02, BR-C03, BR-C04, BR-C05 | G3 — Control |
| BR-P01, BR-P02, BR-P04 | G4 — Reconcile |
| BR-P06, BR-W01, BR-W02, BR-W03, BR-W07 | G5 — Recovery |
| BR-P03, BR-P05, BR-P07, BR-P08, BR-W04, BR-W08 | G6 — Parity |
| BR-P09, BR-W05, BR-W09 | G2/G3/G6 security assertions |
| BR-W06 and host prerequisites for all capabilities | G0 — Preflight |
