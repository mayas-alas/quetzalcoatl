---
name: Deliverable
about: Claim one SCOPE outcome row for a workstream lane.
title: "[deliverable] <OUTCOME-ID>: <short summary>"
labels: ["deliverable"]
assignees: ''
---

## Outcome row

| Field | Value |
|---|---|
| SCOPE ID | e.g. `NET` (from `.AGENTS/SCOPE.md`) |
| Lane | `A` / `B` / `C` (from `.AGENTS/WORKSTREAMS.md`) |
| Acceptance | copy the acceptance line verbatim from `.AGENTS/SCOPE.md` |

## Owned paths

- Agent A: `apps/gnx/**`, `crates/gnx-contracts/**`, root Cargo/release metadata, `README.md`, product docs
- Agent B: `apps/gnx-service/**`, `runtime/**`, platform bundle source, runtime compatibility tests
- Agent C: `apps/gnx-bootstrap/**`, `installer/**`, `tools/**`, operational/validation docs

## Definition of done

- [ ] Change-scoped validators pass
- [ ] `tools/check.ps1 -SourceOnly` green, or blocker recorded in `.AGENTS/TRACKER.md`
- [ ] Evidence recorded in `.AGENTS/EVIDENCE.md` and status updated in `.AGENTS/TRACKER.md` within the PR
- [ ] PR reviewed and approved by a different agent