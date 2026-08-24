# Agent workflow guide

This document describes how the coordinator (main agent) delegates
workstreams to subagents and how subagents claim, execute and hand off
deliverable lanes. It is tool-agnostic: the execution framework, roles and
gamification live in `.AGENTS/agentA/` and apply to any agent backend.

## 1. Agent fleet

The canonical roles are defined in `.AGENTS/agentA/ROLES.md` (three grades:
direction, quality, execution). The table below is the local runtime
connectivity of one host — it is not part of the contract and never changes
lanes, roles or review rules.

| Agent identity | Role | Backend |
|---|---|---|
| `kilo` (main) | Coordinator | orchestrator |
| `pi2` | Orquestador subagent | FreeLLMAPI `/v1` |
| `maya` | Primary assistant | FreeLLMAPI `/v1` |
| `pi` | Primary assistant | FreeLLMAPI `/v1` |
| `pi-claude` | Primary assistant | Anthropic backend |
| `pi-embeddings` | Embedding subagent | `/v1/embeddings` |
| `troubleshoot` | Connectivity subagent | `[IP_ADDRESS]:31415` |

**Backend differences do not create ownership lanes.** Lane ownership comes only
from `.AGENTS/WORKSTREAMS.md`; roles come only from `.AGENTS/agentA/ROLES.md`.

## 2. Workstream lifecycle

### Claiming a workstream

1. Read `.AGENTS/WORKSTREAMS.md` to identify the active workstream and its owner lane.
2. If a new workstream is needed, open an issue using
   `.github/ISSUE_TEMPLATE/workstream-claim.md`.
3. Create a branch `wstream/<lane>/<issue>-<slug>` from `master`.
4. Update `.AGENTS/TRACKER.md` board status to `active` with the branch name.

### Feature → Issue → Spec → Branch flow

For **many features** that must be audited and fitted into workstreams:

1. **Open a `deliverable.md` issue** for each feature (`gh issue create --template deliverable.md`).
2. **Link to / create a `.AGENTS/SPEC.md` section** — the issue references the
   spec doc that defines acceptance criteria, contract invariants, and owned paths.
3. **Coordinator assigns lane + subagent** per `WORKSTREAMS.md` ownership.
4. **Subagent branches** `wstream/<lane>/<issue>-<slug>` from `master`.
5. **Implement → validate → PR** (see handoff template below).

Use `.github/ISSUE_TEMPLATE/finding.md` for discoveries during execution that
need triage (correction/blocker/deliverable). See `.AGENTS/COORDINATOR.md §8`
for the full findings-to-issues flow.

### Executing within a lane

- Work **only** inside the lane's owned paths (listed in `.AGENTS/WORKSTREAMS.md`).
- Preserve every invariant in `.AGENTS/SCOPE.md`.
- Record blockers immediately in `.AGENTS/TRACKER.md` under `Active blockers`.
- Do not revert or overwrite another agent's changes.
- Keep one semantic name; version suffixes and parallel transitional
  implementations are prohibited.

### Change-scoped validation

For each change, run the validators scoped to the changed paths:

```powershell
python tools/validation/platform.py     # Agent B (platform bundle)
python tools/validation/repository.py   # All lanes (taxonomy/inventory)
python tools/validation/contracts.py    # Agent A (protocol/version)
python tools/validation/runtime.py      # Agent B (runtime lock)
python tools/validation/remote_execution.py # Agent B/C (remote exec)
python tools/validation/installer.py    # Agent C (installer)
```

For full source validation (omits physical installer build):

```powershell
tools/check.ps1 -SourceOnly
```

### Handoff to coordinator

When the workstream is complete, the assigned agent opens a PR using the handoff
template from `.AGENTS/WORKSTREAMS.md`:

```text
Workstream: freellmapi-omniroute
Changed paths: platform/tofu/service/*.tf, platform/services/freellmapi/*, ...
Contract impact: <summary>
Checks: <validator results>
Known failures: <if any>
Residual risk: <if any>
Next dependency: <agent C or coordinator step>
```

The PR body must include exact validation evidence (validator output, SHA-256
hashes, or `tools/check.ps1 -SourceOnly` output).

### Review and merge

1. A **different agent** reviews and approves the PR (the author cannot self-approve).
2. The coordinator merges to `master` via PR (no direct pushes).
3. The merge PR must update `.AGENTS/TRACKER.md` (status) and
   `.AGENTS/EVIDENCE.md` (evidence) within the same PR.
4. Release stays local: `installer/build.ps1 -QaSigning` never runs on hosted CI.

## 3. Coordinator delegation flow

When the coordinator orchestrates a multi-wave delivery:

```
Coordinator (pi2 orchestrator)
  ├── Wave 1: Agent B
  │     └── platform bundle services (FreeLLMAPI, OmniRoute)
  │         ├── platform/tofu/service/freellmapi.tf
  │         ├── platform/tofu/service/omniroute.tf
  │         ├── platform/services/freellmapi/{compose.yml,serve.json}
  │         ├── platform/services/omniroute/{compose.yml,serve.json}
  │         ├── platform/manifest.toml
  │         └── platform/platform.lock.json
  │
  ├── Wave 2: Agent C
  │     └── validator updates
  │         ├── tools/validation/platform.py
  │         └── tools/validation/repository.py
  │
  └── Wave 3: Coordinator
        └── integration + evidence
            ├── TRACKER.md update (status: done/blockers)
            ├── EVIDENCE.md update (validator output, checksums)
            └── CHANGELOG.md entry
```

### 3.1 Capacity-aware dispatch

Before delegating, the coordinator consults `.AGENTS/CAPACITY.md`:

1. Read the agent roster — check `Active assignments` vs `Max concurrent workstreams`.
2. Prefer the owning lane (Agent A for `A`-lane, etc.).
3. If the owning lane is at capacity, escalate to `pi2` (orchestrator) which can
   run parallel waves.
4. Mark the assignment in `CAPACITY.md` (increment count, set status to `assigned`).

### 3.2 Delegation command pattern

The coordinator uses the Task tool with `pi2` subagent type to delegate:

```text
Task: Delegate freellmapi-omniroute to Agent B
  subagent_type: pi2
  prompt: >
    Act as Agent B (platform runtime). Implement the freellmapi-omniroute
    workstream from .AGENTS/SPEC.md.
    - Create platform/tofu/service/freellmapi.tf and omniroute.tf
    - Create platform/services/freellmapi/{compose.yml,serve.json}
    - Create platform/services/omniroute/{compose.yml,serve.json}
    - Update platform/manifest.toml with immutable image digests
    - Regenerate platform/platform.lock.json
    - Do NOT use mutable tags; all images pinned by SHA-256
    - Tailscale tags: tag:quetzalcoatl-freellmapi, tag:quetzalcoatl-omniroute
    - Run platform.py and repository.py validators
    - Produce a handoff summary
```

Each wave waits for confirmation from the previous wave before starting.

## 4. Issue-based tracking

Every deliverable maps to one GitHub Issue and one PR:

- **Issue**: One of `deliverable`, `blocker`, or `bug` from `.github/ISSUE_TEMPLATE/`.
- **PR**: Closes exactly one issue. Uses the handoff template in the PR description.
- **Tracker mirror**: Status changes in `.AGENTS/TRACKER.md` must reflect the
  GitHub issue/PR state.

## 5. Branch conventions

| Branch pattern | Purpose |
|---|---|
| `wstream/A/<n>-slug` | Agent A deliverable |
| `wstream/B/<n>-slug` | Agent B deliverable |
| `wstream/C/<n>-slug` | Agent C deliverable |
| `wstream/coordinator/<n>-slug` | Cross-lane integration |

Branch must be created from `master`. Merge only via PR.

## 6. Blocker handling

When a blocker is encountered:

1. Stop work in the affected lane.
2. Open a `[blocker]` issue from `.github/ISSUE_TEMPLATE/blocker.md`.
3. Mirror the blocker into `.AGENTS/TRACKER.md` under `Active blockers`.
4. The blocking lane cannot proceed until the blocker is resolved in a
   different PR.

## 7. Workstream template (for new workstreams)

To add a new workstream, update `.AGENTS/WORKSTREAMS.md` with:

```markdown
### <name> (Agent <X>)

Owner: Agent <X> — <lane>
Spec: `<path/to/spec>`

<description of what the workstream delivers>

Agent <X> owns all changes: <list of changed paths>.
Agent <Y> updates: <list of validator paths>.
Coordinator integrates and records evidence.
```

A spec file should be placed at the referenced path and contain:
- Acceptance criteria (must be testable)
- Contract invariants that must be preserved
- Implementation lanes with owned paths per agent
- Handoff template pre-filled for the specific workstream