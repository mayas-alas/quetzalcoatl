# CORE-01 - Persisted member and status contract

## Objective

Add the durable data model and protocol/status representation required for a member without implementing network discovery or the Proxmox join.

## Required work

- Extend persisted role support from controller-only to `Controller | Member`.
- Add the minimum persisted member identity, controller identity, tailnet, reconciliation stage, and cluster-join state required by `.AGENTS/SCOPE.md`.
- Preserve safe deserialization of existing controller state. Do not add a general migration framework.
- Extend `gnx status --json` contracts with a final member-ready representation: `overall=ready`, `stage=READY`, `cluster.joined=true`, `cluster.quorate=true`, local runtime components `ready`, and OpenTofu/Garage/Forgejo `not_applicable` rather than `pending`.
- Add focused unit tests for round trips, legacy controller state, role immutability helpers, and member status serialization.
- Validate the pinned controller ID as non-empty bounded printable data and require the member hostname to follow the exact `gnx-member-*` contract.

## Non-goals

- No Tailscale peer discovery.
- No `pvecm add` execution.
- No OpenTofu process changes.
- No installer, payload, CI, or documentation edits.

## File ownership

- `crates/gnx-service/src/state.rs`
- `crates/gnx-protocol/src/lib.rs`
- Narrow supporting edits in `crates/gnx-service/src/pipe.rs` or `crates/gnx-cli/` only when compilation requires them; report every such edit.

## Verification

Run formatting plus the affected crate tests, then `cargo test --workspace` if feasible.
