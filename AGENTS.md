# Quetzalcoatl agent contract — 0.1.15

Read these files before modifying the repository:

1. `.AGENTS/README.md`
2. `.AGENTS/SCOPE.md`
3. `.AGENTS/DECISIONS.md`
4. `.AGENTS/TRACKER.md`
5. `.AGENTS/EVIDENCE.md`
6. `.AGENTS/tasks/RELEASE_0.1.15.md`
7. `.AGENTS/tasks/INSTALLER_RECOVERY.md`
8. `.AGENTS/tasks/MEMBER_JOIN.md`
9. `docs/INSTALLER_RECOVERY.md`
10. `docs/MEMBER_MEMBERSHIP.md`

## Non-negotiable constraints

- Preserve exactly four Cargo packages.
- Preserve protocol schema 2 and the existing Named Pipe command set.
- Preserve the persisted-state schema and runtime generation `proxmox-cluster-v2`.
- Keep `gnx status`, `gnx configure` and `gnx restart` behavior compatible.
- Add `gnx -v` and `gnx --version` as local CLI actions only.
- Do not execute dependency MSIs directly from Burn Package Cache.
- Do not introduce arbitrary remote execution or controller-side shell commands.
- Do not add a controller/member API, listener, port, token service, crate or application.
- Keep `installer/build.ps1` as the release entry point.

Changes must close the observed installer or membership risk and must be enforced by source validation or Rust tests.
