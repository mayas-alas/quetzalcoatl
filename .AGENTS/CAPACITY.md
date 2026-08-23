# Agent capacity and dispatch

This file tracks agent workload capacity, availability, and active assignments
to replace ad-hoc coordinator work distribution with a visible dispatch system.

## Agent roster

| Agent | Lane | Mode | Max concurrent workstreams | Status | Active assignments | Last updated |
|---|---|---|---|---|---|---|
| `maya` | coordinator | primary | 3 | available | 0 | 2026-08-23T16:37 |
| `pi2` | coordinator | all | 5 | available | 0 | 2026-08-23T16:37 |
| `pi` | B | primary | 2 | available | 0 | 2026-08-23T16:37 |
| `pi-claude` | A | primary | 2 | available | 0 | 2026-08-23T16:37 |
| `troubleshoot` | C | subagent | 2 | available | 0 | 2026-08-23T16:37 |
| `pi-embeddings` | — | subagent | 1 | available | 0 | 2026-08-23T16:37 |

## Dispatch protocol

When the coordinator needs to assign a workstream:

1. **Check capacity**: Read this file. An agent may not receive new work if
   `Active assignments` >= `Max concurrent workstreams`.
2. **Prefer the owning lane**: Agent A handles `A`-lane work, Agent B handles `B`-lane, etc.
3. **Mark assignment**: Increment `Active assignments` and set `Status` to `assigned`.
4. **Agent acknowledges**: The assigned agent updates `Status` to `in-progress`
   within 30 minutes.
5. **Completion**: On PR merge, the agent decrements `Active assignments` and
   sets `Status` back to `available`.

## Active assignments queue

| Priority | Workstream | Issue # | Assigned agent | Assigned at | Deadline | Status |
|---|---|---|---|---|---|---|
| (none currently) | | | | | | |

## Workstream priority levels

| Level | Meaning | SLA |
|---|---|---|
| P0 | Security, data integrity, production-blocker | 24h |
| P1 | Core functionality, contract invariant | 72h |
| P2 | Documentation, test coverage, minor fixes | 7 days |
| P3 | Refactoring, debt, infrastructure | 14 days |

## Capacity notes

- Subagents (`pi-embeddings`, `troubleshoot`) are specialists with limited
  concurrency — only 1–2 parallel assignments.
- The coordinator should batch P2/P3 work on a single agent to avoid
  context-switching overhead.
- If two agents are at full capacity, the coordinator should escalate to the
  `pi2` orchestrator subagent for parallel wave execution.

## Update log

| Timestamp | By | Change |
|---|---|---|
| 2026-08-23T16:37 | maya | Initial creation — all agents listed as available, 0 active assignments |