# Agent: reconciler recovery

## Ownership

- `crates/gnx-service/src/runtime/reconciler.rs`
- status transitions and persisted runtime checkpoints
- controller/member resume behavior

## Objective

Keep the exact 0.1.11 convergence sequence while making the orchestration boundary explicit and reviewable.

## Invariants

- `runtime/mod.rs` is a facade; `reconciler.rs` owns the sequence.
- Stage names and their order are unchanged unless a migration is explicitly approved.
- Role and controller identity remain immutable after persistence.
- Controller resume verifies the cluster instead of recreating it.
- Member resume uses the pinned controller and existing join checkpoint.
- State schema and runtime generation remain unchanged in 0.1.12.
