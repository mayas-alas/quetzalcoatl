---
name: Workstream Claim
about: Claim an entire workstream for an agent to execute
title: "[workstream] <WORKSTREAM-NAME>: <agent-lane> claim"
labels: ["workstream"]
assignees: ''
---

## Workstream to claim

| Field | Value |
|---|---|
| Workstream name | e.g. `freellmapi-omniroute` (from `.AGENTS/WORKSTREAMS.md`) |
| Spec file | e.g. `.AGENTS/SPEC.md` |
| Owner lane | `A` / `B` / `C` (from `.AGENTS/WORKSTREAMS.md`) |
| Agent identity | e.g. `pi2` (or `maya`, `pi`, `pi-claude`) |

## Scope

List the SCOPE outcome rows this workstream delivers (from `.AGENTS/SCOPE.md`):

- [ ] `BND` — Locked platform bundle
- [ ] `RUN` — Closed platform runner
- [ ] `FND` — Reproducible foundation
- [ ] `STO` — Isolated object storage
- [ ] `FRG` — Sovereign repository entry
- [ ] `OCI` — Isolated build path
- [ ] `SVC` — Generic workload path
- [ ] `NET` — Private service exposure
- [ ] `ADM` — Closed Forgejo administration
- [ ] `REC` — Safe recovery
- [ ] `ARP` — Sole maintenance entry
- [ ] `SUP` — Verifiable supply chain
- [ ] `REL` — Integrated release

## Owned paths (from `.AGENTS/WORKSTREAMS.md`)

### Agent A (product contract)
- `apps/gnx/**`
- `crates/gnx-contracts/**`
- Root Cargo/release metadata
- `README.md`
- Product architecture/contracts documentation

### Agent B (platform runtime)
- `apps/gnx-service/**`
- `runtime/**`
- Platform bundle source
- Runtime compatibility tests

### Agent C (delivery assurance)
- `apps/gnx-bootstrap/**`
- `installer/**`
- `tools/**`
- Operational/validation documentation

## Deliverables for this workstream

List the specific artifacts to produce (from the workstream spec):

- [ ] Path 1
- [ ] Path 2
- [ ] ...

## Validation requirements

- [ ] Change-scoped validators pass (`tools/validation/*.py`)
- [ ] `tools/check.ps1 -SourceOnly` green, or blocker recorded in `.AGENTS/TRACKER.md`
- [ ] Evidence recorded in `.AGENTS/EVIDENCE.md` and status updated in `.AGENTS/TRACKER.md` within the PR

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

## Agent delegation protocol

When this issue is opened, the **coordinator** (Kilo/main agent) will:

1. Create a branch `wstream/<lane>/<issue-number>-<slug>` from `master`
2. Delegate to the assigned subagent (`pi2` for orchestration, or direct execution agent)
3. Subagent works only inside the lane's owned paths
4. Subagent opens PR using the handoff template
5. A different agent reviews and approves
6. Coordinator merges to `master` and updates `.AGENTS/TRACKER.md` and `.AGENTS/EVIDENCE.md`

## Assignment

- **Assignee**: The agent identity that will execute this workstream
- **Reviewer**: A different agent identity (cannot be the same as assignee)