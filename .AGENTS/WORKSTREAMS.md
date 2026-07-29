# Workstreams

The coordinator integrates three disjoint lanes into one delivery.

## Agent A — product foundation

Owns:

- root Cargo/release metadata
- `LICENSE`, `NOTICE`, third-party notices
- `apps/gnx/**`
- `crates/gnx-contracts/**`
- architecture and contract documentation

Delivers `LEG`, `TYP` and the metadata portion of `REL`.

## Agent B — runtime architecture

Owns:

- `apps/gnx-service/**`
- `runtime/**`
- runtime compatibility tests

Delivers `ARC`, `RUN` and runtime portions of `TYP`.

## Agent C — delivery assurance

Owns:

- `apps/gnx-bootstrap/**`
- `installer/**`
- `tools/**`
- operational/validation documentation

Delivers `BRD`, `DEL`, `DOC` and assurance portions of `REL`.

## Coordinator-only paths

- `.AGENTS/**`
- `AGENTS.md`
- `Cargo.lock`
- `CHANGELOG.md`
- integration edits crossing two lanes
- final build and evidence

## Handoff

```text
Workstream:
Changed paths:
Contract impact:
Checks:
Known failures:
Residual risk:
Next dependency:
```

One path has one writer. Cross-lane requirements are handed off; they are not
implemented opportunistically by another owner.
