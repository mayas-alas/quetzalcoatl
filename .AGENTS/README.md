# Delivery contract

`.AGENTS` governs one active delivery: Quetzalcoatl 0.2.1 installer recovery. It is
not release history or a generic handbook.

Read in order:

1. `SCOPE.md` — outcomes, risks and exclusions.
2. `WORKSTREAMS.md` — three non-overlapping ownership lanes.
3. `TRACKER.md` — live status, dependencies and handoffs.
4. `EVIDENCE.md` — executable gates and physical acceptance.

## Protocol

1. Preserve the recorded dirty baseline; never revert unrelated user work.
2. Claim only paths assigned to one workstream.
3. Record cross-workstream requirements in `TRACKER.md`.
4. Use one semantic name and one source of truth.
5. A workstream reaches `review` only after its checks pass.
6. The coordinator integrates all workstreams into one Setup artifact.
7. `done` requires the complete source/build gate and explicit residual risks.

Statuses are exactly `ready`, `active`, `blocked`, `review`, `done`.

Product behavior belongs in code and contract tests. Architecture and operations
belong in `docs/`. Release history belongs in `CHANGELOG.md`. `.AGENTS` is deleted or
reset after the delivery is committed.
