# Agent Management Audit

## Status: 2026-08-23T16:27:35-06:00

> **Superseded (2026-08-24):** The execution framework described below
> (`.AGENTS/agentA/`, role ladder, XP/karma gamification) has been replaced by
> the lean **gauntlet** framework (`.AGENTS/gauntlet/`): master+hot+agent
> branch model, bar-builder-blind-critic loop, badges only, no XP. The audit
> below is kept as history of the superseded system.

## 1. Executive summary

The Quetzalcoatl agent ecosystem currently exhibits **two distinct phases**:

1. **Phase 1 (0.2.41–0.2.42 closure)**: A successful multi-agent delivery where the coordinator (`maya`) orchestrated `pi2` as subagent to claim, execute, and hand off the `freellmapi-omniroute` workstream (Agent B) and validator updates (Agent C). The workstream completed with all six source validators passing.

2. **Phase 2 (current)**: The coordinator ran directly as `maya` for post-completion tasks (merge conflict resolution, documentation, evidence recording, dead code removal). No distinct Agent A/B/C roles are visible in commits  `1171016`–`ba82342`.

**Key finding: There is no incentivization, gamification, performance tracking, or agent-specific recognition system.** All commits are authored by a single identity (`maya@email.gnx` or `[EMAIL]`). While the workflow documentation (`docs/AGENT_WORKFLOW.md`, `.AGENTS/WORKSTREAMS.md`, `.github/ISSUE_TEMPLATE/workstream-claim.md`) now describes a multi-agent process, the **actual execution is single-agent**.

---

## 2. Agent inventory and usage

### Configured agents (from `.kilo/agent/`)

| Agent | Mode | Backend | Usage in git history |
|---|---|---|---|
| `maya` | primary | FreeLLMAPI `/v1` | 100% of commits |
| `pi` | primary | FreeLLMAPI `/v1` | Not observed (no commits) |
| `pi-claude` | primary | Claude | Not observed (no commits) |
| `pi2` | all (orchestrator) | FreeLLMAPI `/v1` | Not observed as distinct commits |
| `pi-embeddings` | — | `/v1/embeddings` | Not observed |
| `troubleshoot` | — | `127.0.0.1:31415` | Not observed |

### Commit authorship analysis

All 20 commits on `master` (from `0e573bf` to `ba82342`) are authored by `maya` (email `maya@email.gnx` or `[EMAIL]`). The `.AGENTS/TRACKER.md` "Per-agent commit history" table claims commits attributed to Agent A, Agent B, and Agent C, but **all share the same git author identity**. There is no mechanism to distinguish which agent performed which work at the git layer.

### Subagent usage (Task tool)

The `pi2` orchestrator subagent type is configured (`mode: all`) but **no evidence of Task tool invocations producing distinct commit authors**. The `freellmapi-omniroute` workstream was likely delegated via `pi2` but all resulting work was committed under `maya`.

---

## 3. Current agent workflow gaps

### 3.1 No agent identity separation
- **Problem**: All commits use a single git author (`maya@email.gnx` / `[EMAIL]`). The multi-agent framework (Agent A/B/C lanes) is documented but not enforced at the tool or git level.
- **Evidence**: `git log --format="%an|%ae"` shows uniform authorship across 20 commits.
- **Impact**: Cannot verify review/approval separation (rule 5: "A different agent reviews and approves; the author cannot self-approve").

### 3.2 No incentivization or gamification
- **Problem**: No reward, recognition, scoring, or progress-tracking system exists for agents.
- **Evidence**: No points, badges, leaderboards, streaks, or performance metrics in `.AGENTS/`, `docs/`, or `.kilo/` configurations.
- **Impact**: Agents have no mechanism to prioritize high-value work, compete constructively, or be motivated beyond explicit instructions.

### 3.3 No workload balancing or queuing
- **Problem**: No central work queue, task prioritization, or load distribution. The coordinator assigns work ad-hoc.
- **Evidence**: `.AGENTS/WORKSTREAMS.md` lists lanes but there is no dispatcher, no priority queue, and no capacity tracking.
- **Impact**: Workstream claims are serial; no parallel execution across lanes despite the framework allowing it.

### 3.4 No progress visibility
- **Problem**: No dashboard, progress bars, or completion metrics for agents.
- **Evidence**: `.AGENTS/TRACKER.md` uses manual status tables (`ready`, `active`, `blocked`, `review`, `done`) but no automated tracking or SLA monitoring.
- **Impact**: Coordinator must manually check each agent's status; no real-time visibility.

### 3.5 No error recovery or retry mechanism
- **Problem**: When a subagent fails (e.g., the Docker-dependent test failure), there is no automatic retry, escalation, or compensation.
- **Evidence**: `check_docker_pipe_contention_missing_pipe` test fails on hosts without Docker, documented as a known blocker (PUB-2) but not automatically retried on a Docker-capable host.
- **Impact**: Work can stall; no graceful degradation.

### 3.6 No skill specialization enforcement
- **Problem**: The `kilo-config` skill is available but not leveraged for agent-specific configuration. No agent is specialized for validation, documentation, or conflict resolution.
- **Evidence**: All agents share the same config paths (`.kilo/agent/*.md`); no per-agent skill assignment.
- **Impact**: Redundant work (e.g., each agent must independently run validators).

---

## 4. Recommendations (tiered)

### Tier 1 — Immediate (within current framework)

#### R1.1: Git author separation per agent
Modify the commit workflow so each agent commits under its own identity:
```
Agent A:    gnx-agent-a@email.gnx   (pi-claude)
Agent B:    gnx-agent-b@email.gnx   (pi)
Agent C:    gnx-agent-c@email.gnx   (troubleshoot or dedicated)
Coordinator: gnx-coordinator@email.gnx (maya/pi2)
```
- Update `.kilo/agent/*.md` with `git_author` and `git_email` fields.
- Modify the commit script in `tools/check.ps1` or add a wrapper to set the correct author per agent.
- This enables verification of review/approval separation (rule 5).

#### R1.2: Lightweight work tracking in `.AGENTS/TRACKER.md`
Add a per-agent workload tracking section:
```markdown
## Agent workload
| Agent | Active workstreams | Last update |
|---|---|---|
| Agent A | 0 | — |
| Agent B | 0 | — |
| Agent C | 0 | — |
| Coordinator | 3 (merge fix, docs, evidence) | 2026-08-23 |
```
- Reduces coordinator overhead.

#### R1.3: Validator auto-run on file changes
Add a `.kilo/hooks/file-change` config (if supported) or a `tools/validate-changed.sh` script that runs only the validators affected by changed files.
- Reduces redundant full-suite runs.

### Tier 2 — Gamification layer (optional, non-blocking)

#### R2.1: Agent scoring system
Add a lightweight scoring mechanism in `.AGENTS/SCOREBOARD.md`:

```markdown
# Agent scoreboard
| Agent | Deliverables | Reviews | Blocked | Streak |
|---|---|---|---|---|
| Agent A | 1 (feat(contracts)) | 0 | 0 | 1 |
| Agent B | 3 (freellmapi services, rename fix, compose alignment) | 0 | 0 | 3 |
| Agent C | 2 (validators, 0.2.42 build) | 0 | 0 | 2 |
| Coordinator | 8 | 4 | 1 | 8 |
```

Rules:
- +10 points per deliverable completed and merged.
- +5 points per PR reviewed/approved.
- +2 points per day streak (consecutive days with activity).
- -5 points per blocker introduced (requires review to reverse).

#### R2.2: Achievement badges
Define milestones as badges in `.AGENTS/BADGES.md`:
- **"First Claim"**: First workstream claimed.
- **"Resolver"**: Resolves a blocker.
- **"Validator"**: Runs all six validators successfully.
- **"Conflict Slayer"**: Resolves >3 merge conflicts in one commit.
- **"Reviewer"**: Reviews a PR from another agent.
- **"Clean Master"**: No merge conflicts introduced.

#### R2.3: Sprint-based goals
Partition delivery into 1-week sprints with agent goals:
```markdown
## Sprint 2026-W34
- Goal: Complete freellmapi-omniroute physical deployment
- Agent B: Deploy 4 LXCs, wire into runtime/deploy
- Agent C: Update platform.py for multi-instance validation
- Agent A: Expose FreeLLMAPI/OmniRoute in CLI status
- Coordinator: Physical verification on Proxmox
```

### Tier 3 — Infrastructure enhancements

#### R3.1: Agent-specific validation profiles
Each agent gets a `tools/validation/<agent>.json` profile specifying which validators to run:
```json
{
  "Agent A": ["contracts.py", "repository.py"],
  "Agent B": ["platform.py", "runtime.py", "remote_execution.py", "repository.py"],
  "Agent C": ["installer.py", "repository.py"]
}
```

#### R3.2: Automated workstream dispatch
A coordinator script (`tools/coordinator/dispatch.py`) that:
1. Reads `.AGENTS/TRACKER.md` for `blocked`/`active` items.
2. Reads `.AGENTS/WORKSTREAMS.md` for lane ownership.
3. Assigns the highest-priority `blocked` item to the next available agent.
4. Opens a `workstream-claim` issue automatically.
5. Creates the branch and commits to TRACKER.

#### R3.3: Cross-agent review enforcement
Modify `tools/check.ps1` to verify that the PR author differs from the reviewer at the git level (using the per-agent git identities from R1.1). Reject merge if same agent authored and reviewed.

---

## 5. Priority assessment

| Recommendation | Complexity | Impact | Priority |
|---|---|---|---|
| R1.1 Git author separation | Low | High | **P0** |
| R1.2 Workload tracking | Low | Medium | **P1** |
| R1.3 Validator auto-scope | Low | Medium | **P1** |
| R2.1 Agent scoring | Medium | Medium | P2 |
| R2.2 Achievement badges | Low | Low-Medium | P2 |
| R2.3 Sprint goals | Low | Low | P2 |
| R3.1 Validation profiles | Low | Low | P2 |
| R3.2 Automated dispatch | High | High | P3 |
| R3.3 Review enforcement | Medium | High | P3 |

---

## 6. Conclusion

The current agent ecosystem is **under-incentivized and under-tracked**. The process framework is well-documented (thanks to the recent `AGENT_WORKFLOW.md` addition), but:

1. **No rewards**: Agents receive no recognition, points, or feedback for completed work.
2. **No competition**: No leaderboard, sprint, or achievement system to motivate efficiency.
3. **No separation**: All commits come from one identity, undermining the multi-agent review process.
4. **No visibility**: The coordinator must manually track every agent's state.

**Immediate action**: Implement R1.1 (git author separation) to enable review-enforcement verification. This is the foundation for any gamification system.

**Secondary action**: Implement R2.1 (scoring) and R2.2 (badges) as a lightweight YAML/JSON file in `.AGENTS/` that agents update themselves after completing work. This adds a feedback loop without infrastructure changes.

**Note**: The `freellmapi-omniroute` workstream (FRE) is currently `blocked` per TRACKER.md (FRE-2) due to a spec/contract gap — the committed service templates are not wired into the runtime deployment path. This is the highest-priority technical blocker to address regardless of gamification.