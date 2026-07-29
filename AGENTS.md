# Quetzalcoatl agent contract — 0.1.17

Read these files before modifying the repository:

1. `.AGENTS/README.md`
2. `.AGENTS/SCOPE.md`
3. `.AGENTS/DECISIONS.md`
4. `.AGENTS/TRACKER.md`
5. `.AGENTS/EVIDENCE.md`
6. `.AGENTS/tasks/RELEASE_0.1.17.md`
7. `docs/REMOTE_EXECUTION.md`
8. `docs/REMOTE_EXECUTION_REVIEW.md`
9. `docs/RUNTIME_LIFECYCLE.md`
10. `docs/HOST_RESOURCE_PROFILE.md`
11. `docs/INSTALLER_RECOVERY.md`
12. `docs/MEMBER_MEMBERSHIP.md`

## Non-negotiable product constraints

- Preserve exactly four Cargo packages.
- Preserve protocol schema 2, the Named Pipe command set and persisted-state schema 2.
- Preserve runtime generation `proxmox-cluster-v2` and payload contract 5.
- Keep the 0.1.15 installer recovery and bounded member-join behavior.
- Compute host CPU, RAM and disk through a closed preflight operation.
- Use one persisted profile for both service `.wslconfig` and Podman Machine creation.
- Resolve a new node only from online controller presence: no controller means controller; one or more controllers means member. Existing members do not affect the decision.
- Apply Tailscale Serve only after the local PVE backend is ready.
- Keep `installer/build.ps1` as the release entry point.

## Remote-execution contract

- Argument vectors describe a closed operation; they must not contain shell syntax.
- Variable data, JSON, configuration payloads and secrets travel through bounded stdin.
- Repository-owned multiline programs run only through a fixed interpreter in stdin mode, such as `sh -s` or `python3 -`.
- Files are used only for durable state or when the consumer requires a path. Durable writes must be bounded, validated and atomic.
- `sh -c`, `bash -c`, redirection, pipelines, caller-provided commands and arbitrary remote argv are prohibited.
- Do not introduce arbitrary resource values, commands, remote execution or new network surfaces.

Any exception requires an explicit decision record, a narrow validator exception and a dedicated regression test. The default exception set is empty.
