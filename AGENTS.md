# Quetzalcoatl agent contract

Read the live framework before changing the repository:

1. `.AGENTS/README.md`
2. `.AGENTS/SCOPE.md`
3. `.AGENTS/WORKSTREAMS.md`
4. `.AGENTS/TRACKER.md`
5. `.AGENTS/EVIDENCE.md`
6. `.AGENTS/agentA/README.md` — the agent-agnostic execution framework
   (roles, closed loop, loop-guard, gamification, resume protocol)

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

## Sticky resume — any agent, any state

The framework is self-contained under `.AGENTS/` and independent of any agent
tool. Before any action, run the resume protocol from
`.AGENTS/agentA/README.md`:

1. Open `.AGENTS/agentA/TRACKING.md` (master table) and find your role card in
   `.AGENTS/agentA/agents/<role>.md`.
2. If a ticket is `started`, resume from its `checkpoint` and update
   `updated_at` + `agent_ping`. Never re-claim without reading the checkpoint.
3. Never sign your own `done`; a grade ≥ 2 role signs it, and `done` without
   `evidence` returns to `verify`.
4. If a ticket cycles (≥ 2 corrections or past its time-box), stop and follow
   the `FLOW.md` loop-guard; record the correction in `CORRECTIONS.md` first.
5. XP/badges/leaderboard are derived by `python/karma.py` from the master
   table; run `python .AGENTS/agentA/python/karma.py --check` after closing a
   ticket.

This contract is the single source of truth. Local agent tooling (fleet
connectivity, prompts) is not part of it: backend differences never create
ownership lanes — lane ownership comes only from `.AGENTS/WORKSTREAMS.md`, and
roles come only from `.AGENTS/agentA/ROLES.md`. A reviewer is any different
agent identity, independent of model or tool.

## Development flow

GitHub Issues and pull requests are the execution layer; `.AGENTS/*` remains the
living contract. One issue maps to one deliverable lane; one PR closes one issue.

**QA-only scope — no production effort:** This repository is under active QA
development. Production release is explicitly blocked (see `.AGENTS/SCOPE.md`
exclusion: hosted CI on another Windows host; and active blocker B40-1:
Smart App Control requires trusted Windows AuthRoot). All workstream subagents
must scope to QA-signed (`-QaSigning`) artifacts only. Do not implement production
signing pipelines, public Internet exposure, or hosted CI runners. Any change
that crosses into production trust surfaces must be recorded as a blocker first.

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

The agent fleet is defined by roles in `.AGENTS/agentA/ROLES.md`, not by any
tool's local configuration. Local runtime connectivity (`.kilo/agent/`,
git-ignored) only selects a backend; it never changes lanes, roles or review
rules. A reviewer is any different agent identity, independent of model or tool.

## Remote execution

Read `docs/CONTRACTS.md` before changing any process that crosses into Podman
Machine or a managed container.

Argument vectors describe closed operations. Variable data uses bounded stdin.
Durable files are GNX-owned, validated and atomic. `sh -c`, `bash -c`, shell-control
syntax in argv, caller-provided commands and arbitrary remote argv are prohibited.

Exceptions require an explicit scope amendment, a narrow validator allowance and a
regression test. The current exception set is empty.
