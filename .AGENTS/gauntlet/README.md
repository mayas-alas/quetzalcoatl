# gauntlet — clean-branch adversarial delivery framework

`gauntlet` is the single execution framework for this repository. It is
tool-agnostic: it depends on no agent runtime, no config directory and no
third-party script. Any agent resumes it with only the plain Markdown under
`.AGENTS/gauntlet/` and the standard git command set.

It replaces the previous `agentA` framework. One taxonomy, no parallel copy.

## Why a gauntlet

Plain review converges on whatever the builder already believes. A gauntlet
loop instead pairs a **builder** with a **separate critic that has fresh
context and real authority to reject**. The critic judges the artifact, not the
story about it. If the work loses, the single largest gap goes back to the
builder and the loop runs again. The loop exits only on a stop rule written
*before* the work started — never on a felt sense of "probably good enough".

## Branch model (clean repo, no feature branches)

| Branch | Role |
|---|---|
| `master` | Always clean and protected. No agent commits to it directly. Only `hot` updates it. |
| `hot` | The single staging branch. The **only** branch allowed to merge into `master`. Accumulates gauntlet-passed changes. Long-lived. |
| `agent/<name>/<slug>` | Short-lived per-agent work branch, created from `hot`. Belongs to exactly one agent and one ticket. Deleted after it merges into `hot`. History is contained to that branch. |

Rules

- No `wstream/...`, `feat/...` or other feature branches. A workstream's files
  exist only on its own agent branch.
- An agent branch is created from `hot`, never from `master` or another
  agent's branch.
- A change reaches `master` only through the gauntlet verdict `PASS`, then
  merge to `hot`, then `hot` to `master`.
- Deleting an agent branch after merge keeps the repo clean; the history stays
  in git.

## Resume protocol — required before any agent acts

1. Read `AGENTS.md` and the live framework (`.AGENTS/README.md`,
   `.AGENTS/SCOPE.md`, `.AGENTS/WORKSTREAMS.md`, `.AGENTS/TRACKER.md`,
   `.AGENTS/EVIDENCE.md`).
2. Read this `README.md`, then `GAUNTLET.md`.
3. Open `BOARD.md` (master table). Find your role's open rows and the
   checkpoint of any `building`/`review` ticket. Never re-claim without
   reading the checkpoint.
4. Update the row's `updated_at` and your branch's tip before acting.

## Files

| File | Purpose |
|---|---|
| `README.md` | Entry point, branch model, resume protocol. |
| `MODEL.md` | Roles, ticket states, badges, corrections. |
| `GAUNTLET.md` | The loop: bar, builder, critic, verdict, rounds, stop rules, loop-guard. |
| `BOARD.md` | Master board (single table) + corrections log. |