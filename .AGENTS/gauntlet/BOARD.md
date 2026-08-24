# BOARD - master gauntlet tracking

Single source of truth. One row per ticket. Vocabulary identical to `MODEL.md`
and `GAUNTLET.md`. Completed history belongs in `.AGENTS/TRACKER.md` and
`CHANGELOG.md`, not here.

## Master table

| ID | task | agent | branch | role | state | bar | round | verdict | largest_gap | evidence | updated_at |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A-018 | Restructure the delivery framework: replace agentA with the lean English gauntlet (bar + builder + blind critic), branch model master+hot+agent, badges, one taxonomy. | orchest. | `hot` (coordinator) | orchestrator | building | The framework is English-lean, single taxonomy, repository.py inventory matches on disk, all references coherent, master clean. **Actual working-tree state:** 13 agentA files staged for deletion, gauntlet/ framework untracked, .AGENTS/WORKSTREAMS.md modified (wstream→agent/ transition pending), .AGENTS/README.md modified, .AGENTS/TRACKER.md modified, .github/ISSUE_TEMPLATE/workstream-claim.md modified (wstream→agent/ transition pending). The actual artifact (what will be committed after Phase 1) is: `git add .AGENTS/gauntlet/ && git add .AGENTS/WORKSTREAMS.md && git add .github/ISSUE_TEMPLATE/workstream-claim.md && git commit`. | 1 | ? | ? | ? | 2026-08-24 |

## Corrections

Recorded here **before** retrying (see `MODEL.md`).

_No corrections on the `gauntlet` framework yet._