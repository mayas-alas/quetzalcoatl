# Quetzalcoatl agent contract

## Mission

Close the existing PoC as the scoped GNX MVP: the same Windows installer converges one controller and two members into a three-node, quorate Proxmox cluster. The controller is the only OpenTofu writer and the only host for Garage and Forgejo.

Before changing code, read:

1. `.AGENTS/SCOPE.md`
2. `.AGENTS/DECISIONS.md`
3. `.AGENTS/TRACKER.md`
4. The assigned task under `.AGENTS/tasks/`
5. `docs/ARCHITECTURE.md` for the normative technical contract

If these sources disagree, stop and report the exact conflict. Do not choose a larger scope.

## Agent execution policy

- Implementation agents are launched with the Codex CLI only.
- Do not spawn or delegate to subagents from a Codex CLI run.
- The primary architect owns scope, task boundaries, integration, tracker scores, and release decisions.
- Modify only the files listed in the assigned task. Report any required cross-task change instead of making it.
- Do not commit, push, dispatch workflows, or mutate remote state unless the assigned prompt explicitly authorizes it.
- Never print, persist, copy, or summarize secret values. Report only whether a required secret is present.

## Scope guard

The functional MVP acceptance topology is exactly:

```text
1 controller + 2 members + quorate Proxmox cluster
```

Member logic should be reusable and must not hard-code `member-1` or `member-2`, but a fourth node and general N-node support are not release requirements for this cycle.

Out of scope: Tauri tray, Headscale, Forgejo Runner, HA/controller election, controller promotion, concurrent first installs, multi-cloud, alternative runtimes, S3 OpenTofu backend, gRPC, advanced UI, general migrations, and remote telemetry.

## Engineering rules

- Preserve the single-writer invariant: members must explicitly deny OpenTofu execution and must never create controller workloads.
- Role selection is one-time and persisted. A restart must never rediscover or change the role.
- Member join must be resumable and idempotent.
- Secrets may move only through the established DPAPI-to-stdin-or-ephemeral-`/run` pattern and must never appear in arguments, logs, state, Compose, or OpenTofu state.
- Proxmox ports stay inside the guest network; never publish them on Windows.
- GitHub Actions with Dockur is compatibility evidence, not the Corosync network acceptance gate.
- The cluster gate requires three consumer Windows 11 hosts in one low-latency site with direct Tailscale paths, zero packet loss, and RTT below 5 ms for every pair.

## Verification

Run the narrowest relevant checks first, then the workspace checks when the task is complete:

```powershell
cargo fmt --all -- --check
cargo test --workspace
```

For installer or payload changes, also run the applicable static validation or build command documented by the task. Never claim a Windows, Dockur, Proxmox, Tailscale, reboot, or cluster test passed without captured evidence.

## Completion report

Return:

- files changed;
- tests run and exact result;
- remaining blockers;
- evidence paths or run URLs without secrets;
- any requested change that was intentionally left out of scope.
