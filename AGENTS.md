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

## Development flow

GitHub Issues and pull requests are the execution layer; `.AGENTS/*` remains the
living contract. One issue maps to one deliverable lane; one PR closes one issue.

1. Open an issue from `.github/ISSUE_TEMPLATE/` (deliverable, blocker or bug)
   and mirror new blockers into `.AGENTS/TRACKER.md`.
2. Create branch `wstream/<lane>/<issue>-<slug>` from `master`.
3. Work only inside the lane's owned paths (`.AGENTS/WORKSTREAMS.md`).
4. Open a PR using the handoff template; record exact validation evidence
   (`tools/check.ps1 -SourceOnly` or the change-scoped validators).
5. A different agent reviews and approves; the author cannot self-approve.
6. Merge to `master` only via PR. Update `.AGENTS/TRACKER.md` status and
   `.AGENTS/EVIDENCE.md` inside the same PR.
7. Release stays local: `installer/build.ps1 -QaSigning` never runs on hosted CI
   (SCOPE excludes hosted CI on another Windows host) and private keys never
   enter the repository.

Local agent fleet (`.kilo/agent/`, git-ignored): `maya`, `pi` and `pi-claude`
are primary assistants on different backends; `pi2` orchestrates and delegates;
`pi-embeddings` and `troubleshoot` are subagents. Backend differences do not
create ownership lanes — lane ownership comes only from
`.AGENTS/WORKSTREAMS.md`. A reviewer is any different agent identity,
independent of model.

## Remote execution

Read `docs/CONTRACTS.md` before changing any process that crosses into Podman
Machine or a managed container.

Argument vectors describe closed operations. Variable data uses bounded stdin.
Durable files are GNX-owned, validated and atomic. `sh -c`, `bash -c`, shell-control
syntax in argv, caller-provided commands and arbitrary remote argv are prohibited.

Exceptions require an explicit scope amendment, a narrow validator allowance and a
regression test. The current exception set is empty.
