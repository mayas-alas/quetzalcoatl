# Workstreams

Three lanes may work concurrently; one path has one writer.

## Agent A — product contract

Owns root Cargo/release metadata, `apps/gnx/**`, `crates/gnx-contracts/**`,
`README.md` and product architecture/contracts documentation.

Delivers `PCT`, `ADM`, product `SUP`/`REL` and CLI/tray platform status. Rust may
name only the scoped `gnx-admin` bootstrap contract; it must not introduce other
service identities or IaC resources.

## Agent B — platform runtime

Owns `apps/gnx-service/**`, `runtime/**`, platform bundle source and runtime
compatibility tests.

Delivers `BND`, `RUN`, `FND`, `STO`, `FRG`, `OCI`, `SVC`, `NET` and the runtime
half of `REC`. The bundle is one semantic implementation; no copied historical
payload or transitional service path is permitted.

## Agent C — delivery assurance

Owns `apps/gnx-bootstrap/**`, `installer/**`, `tools/**` and
operational/validation documentation.

Delivers installer `REC`, `ARP`, `SUP`, `REL`, bundle validation and the physical
upgrade/repair/restart acceptance.

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
