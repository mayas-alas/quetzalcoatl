# Business requirements — GNX 0.3.1

This document is the product contract for the clean 0.3.1 PoC. Architecture and
implementation choices must trace back to these requirements. A component name
is not a business capability.

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
| BR-P06 | G5 — Recovery |
| BR-P03, BR-P05, BR-P07, BR-P08 | G6 — Parity |
| Host prerequisites for all capabilities | G0 — Preflight |
