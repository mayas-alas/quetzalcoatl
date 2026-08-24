# Coordinator — task delegation and workstream orchestration

This document defines how the **Coordinator** (the agent acting as `maya` or a
primary assistant on the orchestration backend) builds, assigns, and delegates
workstream tasks to subagents. It applies to the `pi` / `pi-claude` / `pi2`
fleet and any delegated `task` agent.

## 1. Workstream model

The repository has three persistent lanes (see `.AGENTS/WORKSTREAMS.md`):

| Lane | Owner paths | Delivers |
|---|---|---|
| **Agent A** — product contract | `apps/gnx/**`, `crates/gnx-contracts/**`, root `Cargo.*`, `README.md`, product docs | `PCT`, `ADM`, `SUP`/`REL`, CLI/tray platform status |
| **Agent B** — platform runtime | `apps/gnx-service/**`, `runtime/**`, `platform/**`, runtime tests | `BND`, `RUN`, `FND`, `STO`, `FRG`, `OCI`, `SVC`, `NET`, runtime `REC` |
| **Agent C** — delivery assurance | `apps/gnx-bootstrap/**`, `installer/**`, `tools/**`, operational docs | installer `REC`, `ARP`, `SUP`, `REL`, bundle validation, physical upgrade/repair |
| **Coordinator** | `.AGENTS/**`, `AGENTS.md`, `Cargo.lock`, `CHANGELOG.md`, cross-lane + physical host + final evidence | Integration, merge, push, release, blockers |

One path has one writer. A subagent **claims** a lane, not the whole repo.

## 2. Lifecycle: how the coordinator assigns a task

### Step 1 — Define the workstream

When a feature or fix is requested:
1. Read `.AGENTS/README.md`, `.AGENTS/SCOPE.md`, `.AGENTS/WORKSTREAMS.md`,
   `.AGENTS/TRACKER.md`, `.AGENTS/EVIDENCE.md`.
2. Determine which lane owns the changed paths.
3. If the spec does not exist, **write it first** in `.AGENTS/SPEC.md` (or
   `.AGENTS/<name>-SPEC.md`) with: scope, changed paths, contract impact,
   acceptance criteria, validator gates, and a handoff template.

### Step 2 — Record it in TRACKER.md

Add a row to the Board table and create a GitHub Issue (if public scope):

```
| <ID> | <status> | <evidence pointer> |
```

Set status = `queued` until a subagent claims it.

### Step 3 — Delegate to a subagent

Use the `task` tool with a **detailed prompt** containing:

```text
You are <Agent A|Agent B|Agent C>. Your owned paths are <paths>.
Workstream: <name> (spec: .AGENTS/SPEC.md)
Task: <concrete deliverable>
Constraints: <list of invariants from SCOPE.md that apply>
Acceptance: <exact check command + expected output>
Deliverable: <what to return: commit + hash + validator output>
```

Do NOT delegate to yourself (the coordinator). The coordinator **orchestrates**;
a different agent identity must **execute**.

### Step 4 — Review the handback

When the subagent returns:
1. Verify the changed paths are within lane ownership.
2. Verify the acceptance check command output matches expectations.
3. Verify no preserved invariant was violated (private keys, credentials,
   mutable tags, PVE secrets in Forgejo, `installer/build.ps1` as release entry).
4. If passing: merge the subagent's work into the integration branch, record
   evidence in `.AGENTS/EVIDENCE.md`.
5. If failing: return to the subagent with specific failure output; do **not**
   silently override another agent's work.

## 3. Task prompt template for subagents

Copy-paste this structure when invoking `task`:

```text
You are <AgentName> (platform runtime lane). Your owned paths: <exact paths>.

**Workstream**: <name> — full spec in `.AGENTS/SPEC.md`

**[Task]**
<one-paragraph concrete deliverable>

**[Constraints]** (from .AGENTS/SCOPE.md — do not violate)
- <invariant 1>
- <invariant 2>
- ...

**[Acceptance criteria]**
- <check 1 + expected result>
- <check 2 + expected result>

**[Deliverable]**
Return: summary of changed files, commit hash (if you can commit), and full
output of the acceptance check command.
```

## 4. Branch and commit hygiene

- The coordinator owns `master` integration. Subagents work in the working tree
  directly (no separate branch needed in this repo model).
- Each lane's changes are committed under the appropriate agent identity:
  - Agent A: `feat(contracts): ...`
  - Agent B: `feat(runtime): ...` or `fix(runtime): ...`
  - Agent C: `feat(delivery): ...` or `fix(delivery): ...`
- The coordinator commits: `chore(coord): ...` or `chore(release): ...`.
- One semantic commit per change. No force-push of another lane's work.

## 5. Physical host mutation (reserved for coordinator)

Only the coordinator may:
- Run elevated `installer/build.ps1 -QaSigning`.
- Execute the QA Smart App Control policy transition.
- Push to GitHub, create releases, attach zip artifacts.
- Mutate the Proxmox controller or LXC workload directly.

Subagents may **prepare** the files but the coordinator **executes** the
physical step and records the evidence.

## 6. Release entry point

`installer/build.ps1` is always the release entry point (never moved or renamed).
QA-signed builds use `-QaSigning`; unsigned dev builds use `-AllowUnsigned`.
The coordinator always runs the final build + release upload.

## 7. Handoff template (for TRACKER.md)

```text
Workstream:
Changed paths:
Contract impact:
Checks:
Known failures:
Residual risk:
Next dependency:
```

## 8. Findings-to-Issues flow

Discoveries during execution must be persisted via `.github/ISSUE_TEMPLATE/finding.md`
and mirrored into the `.AGENTS/TRACKER.md` **Workstream findings log**.

When a subagent or the coordinator encounters a finding:

1. **Open a finding issue** using `gh issue create --title "[finding] ..." --template finding.md`
   - Fill: discovered-by, context, where, what observed, reproducer, expected vs actual
2. **The coordinator triages** into one of:
   - `deliverable` — new feature work (→ `.AGENTS/SPEC.md` section + branch `agent/<name>/<slug>` from `hot`)
   - `correction` — cleanup/dead-code/docs (→ direct commit by owning lane)
   - `blocker` — invariant at risk or physical gate (→ active blockers table, must be resolved before proceeding)
   - `wontfix` / `duplicate` — explicitly closed
3. **Add a row to the Findings log** in `.AGENTS/TRACKER.md` with: Date, Agent, Workstream, Finding summary, Lane, Issue #, TRACKER row, Triage.
4. **Link the issue number** in the Board/Blockers/Resolved table rows.
5. **Close the finding issue** when the associated TRACKER row reaches `done` (coordinator does this after verification).

This ensures every corrective observation is:
- **Auditable** over time (the log grows, never overwrites)
- **Actionable** (assigned lane + subagent)
- **Verifiable** (validator output in PR + evidence)
- **Persistent** (survives across sessions in both `.AGENTS/` and GitHub)

**Field mapping** (finding.md → TRACKER.md):
| Issue field | TRACKER column |
|---|---|
| Date (UTC) | Date (UTC) |
| Discovered by | Agent |
| Workstream | Workstream |
| Where / What | Finding (summarized) |
| Triage → Lane | Lane |
| Issue number | Issue |
| Triage reference | TRACKER row + Triage |
