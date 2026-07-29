# Workstreams

Three lanes may work concurrently; one path has one writer.

## Agent A — product foundation

Owns root Cargo/release/legal metadata, `apps/gnx/**`,
`crates/gnx-contracts/**`, `README.md` and architecture/contracts documentation.

Delivers `LEG`, product `ARC`/`REL` and CLI/tray contracts.

## Agent B — runtime lifecycle

Owns `apps/gnx-service/**`, `runtime/**` and runtime compatibility tests.

Delivers `RUN`, `REC` and the service half of `UNS`/`RBT`.

## Agent C — delivery assurance

Owns `apps/gnx-bootstrap/**`, `installer/**`, `tools/**` and operational/validation
documentation.

Delivers `BRD`, `ARP`, installer `REL`, and end-to-end `UNS`/`RBT`.

## Coordinator-only

Owns `.AGENTS/**`, `AGENTS.md`, `Cargo.lock`, `CHANGELOG.md`, cross-lane
integration, physical host mutations and final evidence.

## Handoff template

```text
Workstream:
Changed paths:
Contract impact:
Checks:
Known failures:
Residual risk:
Next dependency:
```
