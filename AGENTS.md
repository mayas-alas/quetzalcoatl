# Quetzalcoatl agent contract — 0.1.14

Read these files before modifying the repository:

1. `.AGENTS/README.md`
2. `.AGENTS/SCOPE.md`
3. `.AGENTS/DECISIONS.md`
4. `.AGENTS/TRACKER.md`
5. `.AGENTS/EVIDENCE.md`
6. `.AGENTS/tasks/RELEASE_0.1.14.md`
7. `.AGENTS/tasks/STRUCTURAL_REFACTOR.md`
8. `docs/ARCHITECTURE.md`
9. `docs/REMOTE_EXECUTION.md`
10. `docs/TARGET_0.2.md`

## Non-negotiable constraints

- Preserve the four Cargo packages.
- Preserve protocol schema 2 and the existing Named Pipe.
- Preserve `gnx status`, `gnx configure` and `gnx restart`.
- Preserve state schema, runtime generation and payload contract.
- Preserve controller/member and cluster behavior.
- Do not introduce arbitrary remote execution.
- Do not add GitHub Actions, OpenTofu, tray UI, a new service or a new crate.
- Keep `installer/build.ps1` as the release entry point.

Changes must improve a demonstrated boundary and must be covered by static validation or Rust tests.
