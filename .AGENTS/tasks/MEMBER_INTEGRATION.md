# INT-01 - Integrate the member runtime and secure PVE join

## Objective

Connect the reviewed deterministic member state/runtime to the reviewed `gnx-pve-cluster-create join` payload so a member resumes safely and reaches the final status contract.

## Required work

- Persist a member transition from `NotStarted` to `Joining` before invoking the join and from `Joining` to `Joined` only after the payload returns and verifies `PVE_JOIN=ready`.
- A restart in either `Joining` or `Joined` must rerun the idempotent join/verification against the same pinned controller; it must never rediscover or switch role/controller.
- Reload the DPAPI-protected PVE password only for the join, send controller IP/hostname, member IP/hostname, and password through stdin, zeroize every in-memory input buffer, and never include a secret in argv/status/state/logs.
- Map the payload's stable network preflight failures to `CLUSTER_NETWORK_PREFLIGHT_FAILED`; use `PVE_JOIN_FAILED` for other join failures. Preserve `CONTROLLER_UNAVAILABLE` and use `TAILSCALE_DIRECT_PATH_REQUIRED` for a relayed/unavailable controller path.
- After successful join, persist `Joined` and final `READY`, return the CORE-01 member status with joined/quorate true, and keep OpenTofu/Garage/Forgejo `not_applicable`.
- Add an explicit production guard before OpenTofu execution that returns `MEMBER_OPENTOFU_DENIED` for a member before launching any process. Test the guard directly.
- Validate legal member stage/join-state combinations without breaking legacy controller state.
- Add deterministic tests for stage transitions, restart/resume, final status, error mapping, secret-free join input construction, and the OpenTofu denial.

## Non-goals

- No installer, CLI, workflow, service UI, arbitrary controller election, promotion, or fourth-node acceptance changes.
- No alternate credential channel or network fallback.
- No physical-lab or GitHub dispatch.

## File ownership

- `crates/gnx-service/src/runtime_gate.rs`
- `crates/gnx-service/src/state.rs`
- `crates/gnx-protocol/src/lib.rs` only if the existing final member status contract has a proven gap; report before changing it.
- Do not edit the already reviewed payload script or manifest; consume their exact stdin/stdout contract.
