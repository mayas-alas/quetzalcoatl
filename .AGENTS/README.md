# Delivery contract

`.AGENTS` governs the active Quetzalcoatl 0.2 platform-foundation stabilization.
It is not release history; completed historical changes belong in
`CHANGELOG.md`.

Read in order:

1. `SCOPE.md` — outcomes, invariants and exclusions.
2. `WORKSTREAMS.md` — three non-overlapping ownership lanes.
3. `TRACKER.md` — current status, dependencies and blockers.
4. `EVIDENCE.md` — executable and physical acceptance.
5. `agentA/README.md` — the agent-agnostic execution framework: roles,
   closed claim→do→verify→record loop, loop-guard, gamification (XP/badges)
   and the resume protocol any agent runs before acting.

## Protocol

1. Preserve the recorded clean baseline and never revert unrelated user work.
2. Claim only paths assigned to one workstream. The coordinator may execute
   workstreams sequentially when the delivery is assigned to one agent.
3. Record cross-workstream requirements in `TRACKER.md`.
4. Use one semantic name and one implementation; transitional copies are
   prohibited.
5. Foundation source remains outside the Rust domain and is integrated as one
   signed, locked platform bundle.
6. A workstream reaches `review` only after its checks pass.
7. The coordinator integrates one Setup artifact and records exact evidence.
8. `done` requires source gates plus physical acceptance; neither implies the
   other.
9. The framework is tool-agnostic and lives only under `.AGENTS/`. No agent
   tool, runtime config or local fleet directory is part of the contract.
10. Every agent resumes from `agentA/TRACKING.md` before acting: read the
    checkpoint of any `started` ticket, never re-claim without it, and update
    `updated_at` + `agent_ping`. This makes any session restart from any state
    deterministic.

Statuses are exactly `ready`, `active`, `blocked`, `review`, `done`.
