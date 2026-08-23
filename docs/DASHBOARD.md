# Agent progress dashboard

This dashboard provides real-time visibility into agent workload, progress,
and completion status. It is updated automatically by agents after each
work item, and manually by the coordinator during dispatch.

## Active workstreams

| Workstream | Issue | Lane | Agent | Status | Progress | Updated | Blockers |
|---|---|---|---|---|---|---|---|
| freellmapi-omniroute | # — | B | Agent B | **blocked** | 100% (source) | 2026-08-23 | FRE-2: spec/contract gap, physical deployment pending |
| 0.2.42 release | # — | coordinator | maya | done | 100% | 2026-08-23 | B40-1: Smart App Control requires trusted signing |
| QA trust bootstrap | B40-1 | coordinator | maya | done | 100% | 2026-08-23 | Resolved — QA-off authorized |
| Merge conflict resolution | MRK-1 | coordinator | maya | done | 100% | 2026-08-23 | Resolved |
| Dead code removal | — | coordinator | maya | done | 100% | 2026-08-23 | Resolved |

## Agent activity (last 24h)

| Agent | Commits | Issues opened | PRs opened | Reviews given | Currently busy |
|---|---|---|---|---|---|
| maya | 6 | 0 | 0 | 0 | available |
| pi2 | 0 | 0 | 0 | 0 | available |
| pi | 0 | 0 | 0 | 0 | available |
| pi-claude | 0 | 0 | 0 | 0 | available |
| troubleshoot | 0 | 0 | 0 | 0 | available |
| pi-embeddings | 0 | 0 | 0 | 0 | available |

## Delivery velocity

### Commit cadence (per agent, 7-day window)

```
maya: ████████████████████ (20 commits, 100%)
pi2:  (0 commits)
pi:   (0 commits)
pi-claude: (0 commits)
```

### Workstream completion rate

| Sprint | Deliverables claimed | Deliverables completed | Completion % |
|---|---|---|---|
| 2026-W34 | 4 | 4 | 100% |

## Agent performance metrics

| Agent | Avg. source gate pass time | Reviews performed | Blockers resolved | Streak (days active) |
|---|---|---|---|---|
| maya | 35 min | 0 | 4 (MRK-1, RTM-1, PUB-1, PUB-2) | 1 |
| pi2 | — | — | — | 0 |
| pi | — | — | — | 0 |
| pi-claude | — | — | — | 0 |
| troubleshoot | — | — | — | 0 |
| pi-embeddings | — | — | — | 0 |

## Backlog priority

| Priority | Count | Items |
|---|---|---|
| P0 | 1 | B40-1 (Smart App Control) |
| P1 | 2 | PHY-1 (physical deployment), FRE-2 (contract gap) |
| P2 | 2 | OCI-1 (image publication), SEC-1 (key rotation) |
| P3 | 0 | — |

## How to update this dashboard

Agents update their row after completing work:
1. Increment commit count in "Agent activity".
2. Update workstream status in "Active workstreams".
3. Update performance metrics.
4. Add new workstreams/blocked items to the appropriate tables.
5. Commit with message: `docs(dashboard): update after <workstream>``.

The coordinator updates the "Backlog priority" and "Delivery velocity" sections
weekly.

---
*Last updated: 2026-08-23T16:37:13-06:00*