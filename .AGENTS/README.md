# Delivery contract

`.AGENTS` governs the active Quetzalcoatl 0.2 platform-foundation stabilization.
It is not release history; completed historical changes belong in
`CHANGELOG.md`.

Read in order:

1. `SCOPE.md` — outcomes, invariants and exclusions.
2. `WORKSTREAMS.md` — three non-overlapping ownership lanes.
3. `TRACKER.md` — current status, dependencies and blockers.
4. `EVIDENCE.md` — executable and physical acceptance.

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

Statuses are exactly `ready`, `active`, `blocked`, `review`, `done`.
