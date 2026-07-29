# Quetzalcoatl agent contract

Read the live framework before changing the repository:

1. `.AGENTS/README.md`
2. `.AGENTS/SCOPE.md`
3. `.AGENTS/WORKSTREAMS.md`
4. `.AGENTS/TRACKER.md`
5. `.AGENTS/EVIDENCE.md`

Then read only the product documentation relevant to the claimed deliverable.

## Required behavior

- Work only inside a claimed deliverable and its owned paths.
- Preserve every invariant in `.AGENTS/SCOPE.md`.
- Stop and record a blocker before changing packages, schemas, network surfaces or
  compatibility behavior outside the active scope.
- Keep one semantic taxonomy; version suffixes and parallel transitional
  implementations are prohibited.
- Do not revert or overwrite another agent's changes.
- Record blockers, handoffs and exact validation evidence.
- Keep `installer/build.ps1` as the release entry point.

## Remote execution

Read `docs/CONTRACTS.md` before changing any process that crosses into Podman
Machine or a managed container.

Argument vectors describe closed operations. Variable data uses bounded stdin.
Durable files are GNX-owned, validated and atomic. `sh -c`, `bash -c`, shell-control
syntax in argv, caller-provided commands and arbitrary remote argv are prohibited.

Exceptions require an explicit scope amendment, a narrow validator allowance and a
regression test. The current exception set is empty.
