---
name: Blocker
about: Record a new delivery blocker or track an existing TRACKER.md blocker.
title: "[blocker] <ID>: <short summary>"
labels: ["blocker"]
assignees: ''
---

## Blocker

| Field | Value |
|---|---|
| ID | e.g. `B40-1`, or a new `B##-N` |
| Finding | what was observed |
| Required resolution | what must happen to unblock |
| Lane | `A` / `B` / `C` / coordinator |

Mirror this into `.AGENTS/TRACKER.md` `Active blockers` when opened. The PR that
resolves it moves the row to `Resolved blockers` with evidence.