# Workstreams

Three lanes may work concurrently; one path has one writer.

## Issue-First mandate

**Every workstream, feature, bug, blocker, or finding MUST have a GitHub Issue
before any work begins.** The Issue is the source of truth; the `.AGENTS/*`
framework documents *how* work is done.

### Mandatory Issue linkage

| Artifact | Must reference GitHub Issue # |
|---|---|
| Workstream (in `.AGENTS/WORKSTREAMS.md`) | Yes — column `Issue` |
| Board row (in `.AGENTS/TRACKER.md`) | Yes — column `Issue` |
| Active blocker row | Yes — column `Issue` |
| Finding row (in Findings log) | Yes — column `Issue` |
| Branch name | `agent/<name>/<slug>` from `hot` |
| PR title / body | Must include `Closes #<issue>` |

If a workstream lacks an Issue, it is invalid and will not be merged.

## Agent A — product contract

Owns root Cargo/release metadata, `apps/gnx/**`, `crates/gnx-contracts/**`,
`README.md` and product architecture/contracts documentation.

Delivers `PCT`, `ADM`, product `SUP`/`REL` and CLI/tray platform status. Rust may
name only the scoped `gnx-admin` bootstrap contract; it must not introduce other
service identities or IaC resources.

## Agent B — platform runtime

Owns `apps/gnx-service/**`, `runtime/**`, platform bundle source and runtime
compatibility tests.

Delivers `BND`, `RUN`, `FND`, `STO`, `FRG`, `OCI`, `SVC`, `NET` and the runtime
half of `REC`. The bundle is one semantic implementation; no copied historical
payload or transitional service path is permitted.

## Agent C — delivery assurance

Owns `apps/gnx-bootstrap/**`, `installer/**`, `tools/**` and
operational/validation documentation.

Delivers installer `REC`, `ARP`, `SUP`, `REL`, bundle validation and the physical
upgrade/repair/restart acceptance.

## Coordinator-only

Owns `.AGENTS/**`, `AGENTS.md`, `Cargo.lock`, `CHANGELOG.md`, cross-lane
integration, physical host mutations and final evidence.

## Active workstreams (additional)

### freellmapi-omniroute (Agent B)

Owner: Agent B — platform runtime
Spec: `.AGENTS/SPEC.md`
Issue: #2 (closed, delivered)
ticket: A-003 (completed)

Adds two new managed services, each deployed as 2 LXC instances via OpenTofu service
root, with Tailscale sidecar HTTPS exposure:

| Service | Repo | Instances | VMID range | LXC name pattern | Tailscale tag |
|---|---|---|---|---|---|
| FreeLLMAPI | github.com/tashfeenahmed/freellmapi | 2 | 300-301 | gnx-freellmapi-{1,2} | tag:quetzalcoatl-freellmapi |
| OmniRoute | github.com/diegosouzapw/OmniRoute | 2 | 302-303 | gnx-omniroute-{1,2} | tag:quetzalcoatl-omniroute |

Agent B owns all changes: `platform/tofu/service/*.tf`, `platform/services/freellmapi/*`,
`platform/services/omniroute/*`, `platform/manifest.toml`, `platform/platform.lock.json`.

Agent C updates: `tools/validation/platform.py`, `tools/validation/repository.py`.

Coordinator integrates and records evidence.

### deepseek-dsh (Agent B)

Owner: Agent B — platform runtime
Spec: `.AGENTS/SPEC.md`
Issue: #11 (open)
ticket: A-004 (active)

Adds one new managed service deployed as 1 LXC instance via OpenTofu service root,
with Tailscale sidecar HTTPS exposure:

| Service | Repo | Instances | VMID range | LXC name pattern | Tailscale tag |
|---|---|---|---|---|---|
| DeepSeek Harness | `npx @deepseek-ai/dsh` (community image `alliot/deepseek-harness`) | 1 | 304 | gnx-deepseek-dsh-1 | tag:quetzalcoatl-deepseek-dsh |

Agent B owns all changes: `platform/services/deepseek-dsh/*`, `platform/manifest.toml`,
`platform/platform.lock.json`.

Agent C updates: `tools/validation/platform.py`, `tools/validation/repository.py`.

Coordinator integrates and records evidence.

### phy-deployment (Coordinator)

Owner: Coordinator
Issue: #5 (open)
ticket: A-005 (active)

Physical execution of 0.2.42 install/upgrade/repair on the Proxmox controller,
and deployment of 4 new FreeLLMAPI/OmniRoute LXCs via OpenTofu.
Verify Tailscale HTTPS access and health probes for all 4 new services.

### oci-images (Agent A / OCI lane)

Owner: Agent A — product contract
Issue: #7 (open)
ticket: A-006 (queued)

Build and publish FreeLLMAPI and OmniRoute OCI images by digest via the
Forgejo template + dedicated runner. Reconcile digests in manifest/compose.
DeepSeek Harness image (OCI-2) also pending dedicated runner build.

## Workstream process

Agent delegation, issue tracking and branch conventions are documented in
`docs/AGENT_WORKFLOW.md`. Each agent:

1. Opens a workstream-claim issue from `.github/ISSUE_TEMPLATE/workstream-claim.md`.
2. Creates branch `agent/<name>/<slug>` from `hot`. No feature branches; only
   `hot` updates `master`.
3. Works only inside the lane's owned paths (`.AGENTS/WORKSTREAMS.md`).
4. Runs change-scoped validators and records results.
5. Opens a PR using the handoff template below; the coordinator reviews.
6. The PR updates `.AGENTS/TRACKER.md` (status), `.AGENTS/gauntlet/BOARD.md` (row)
   and `.AGENTS/EVIDENCE.md` (evidence).
7. Merge into `hot` (the only branch that updates `master`), then `hot` →
   `master`, only via PR. Authors never self-approve; the gauntlet verdict is a
   fresh-context critic's `PASS`.

## Gauntlet cross-lane integration

The gauntlet framework (`.AGENTS/gauntlet/`) governs every workstream. Each
workstream maps to a gauntlet ticket (`A-<NNN>`) on
`.AGENTS/gauntlet/BOARD.md`.

### Role mapping per lane

| Lane | Builder | Critic |
|------|---------|--------|
| Agent A — product contract | builder, lane A | critic from another lane, fresh context |
| Agent B — platform runtime | builder, lane B | critic from another lane, fresh context |
| Agent C — delivery assurance | builder, lane C | critic from another lane, fresh context |

### Mandatory gauntlet linkage

| Artifact | Must reference gauntlet ticket `A-<NNN>` |
|---|---|
| Workstream (in `.AGENTS/WORKSTREAMS.md`) | Yes — column `ticket` |
| Board row (in `.AGENTS/gauntlet/BOARD.md`) | Yes — column `ID` |
| Board row (in `.AGENTS/TRACKER.md`) | Yes — column `ticket` |
| Active blocker row | Yes — column `ticket` |
| PR title / body | Must include `Closes #<issue> A-<NNN>` |
| Branch name | `agent/<name>/<slug>` from `hot` |

### Gauntlet gates per PR

- The bar is written before any work starts; it is named, fetchable and losable,
  and it is never renegotiated after the artifact exists.
- `PASS`/`REJECT` comes only from a critic with fresh context, never from the
  builder; `REJECT` must name exactly one reproducible gap.
- On `PASS` the change merges to `hot` with change-scoped validator output as
  evidence; `hot` is the only branch that updates `master`.
- Loop-guard: if a workstream cycles (≥2 rejections or time-box overrun), the
  gauntlet loop-guard trips; the correction is recorded on `BOARD.md` and the
  orchestrator decides `parked`/`blocked`.
- Badges are the only reward layer (see `.AGENTS/gauntlet/MODEL.md`); there is
  no XP or leaderboard.

## Handoff template

```text
Workstream:
Changed paths:
Contract impact:
Checks:
Known failures:
Residual risk:
Next dependency:
```
