You are the independent INT-01 acceptance reviewer. Do not spawn subagents and do not edit, commit, stage, push, dispatch, or mutate remote state.

Review only the current uncommitted member/Proxmox integration in:

- crates/gnx-protocol/src/lib.rs
- crates/gnx-service/src/state.rs
- crates/gnx-service/src/runtime_gate.rs
- runtime/payload-v1/bin/gnx-pve-cluster-create
- runtime/payload-v1/manifest.json

Read AGENTS.md, .AGENTS/SCOPE.md, .AGENTS/DECISIONS.md, .AGENTS/tasks/MEMBER_INTEGRATION.md, and .AGENTS/tasks/PVE_JOIN.md first. Treat `git diff` as the proposed patch.

Audit concrete correctness and security only: NotStarted -> Joining persistence before invocation; Joining/Joined reboot verification against the same pinned controller; exact five-line stdin and exact success stdout; DPAPI password lifetime and every duplicate/buffer being zeroized; no secret in argv/state/status/error/log; stable failure mapping; the direct-path requirement; controller behavior regression; member OpenTofu denial before process launch; legal state/stage pairs; and whether tests actually exercise these contracts. Check the payload/manifest contract and run narrow checks when useful.

Report only actionable P0/P1/P2 findings with exact file and line references. If none exist, report CLEAN and list the checks you personally ran. Do not invent fourth-node, HA, cloud, alternate-election, unattended-secret, or non-MVP scenarios.
