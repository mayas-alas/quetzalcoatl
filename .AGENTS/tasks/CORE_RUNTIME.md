# CORE-02 - Deterministic member discovery and orchestration

## Objective

Replace `MEMBER_INCREMENT_DEFERRED` with the bounded, deterministic role/member flow after CORE-01 is integrated.

## Required work

- Parse and filter Tailscale peer data without logging secrets.
- Exclude self, expired peers, service sidecars, and peers without the exact GNX product tag.
- Implement the topology matrix from `.AGENTS/SCOPE.md` and stable errors for no controller, multiple controllers, unsupported topology, unavailable controller, and non-direct paths.
- Persist the chosen role/controller exactly once and make retries resume the recorded stage.
- Produce member status using the CORE-01 contract.
- Add deterministic tests for every topology row, filtering rule, retry, and immutability behavior.

## Non-goals

- No arbitrary N-node support.
- No payload script or installer changes.
- No Proxmox credential transport implementation.
- No GitHub workflow edits.

## File ownership

- `crates/gnx-service/src/runtime_gate.rs`
- Narrow supporting service files only when required; report them before broadening the patch.
