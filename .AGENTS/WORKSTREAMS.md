# Workstreams

Three lanes may work concurrently; one path has one writer.

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

## Workstream process

Agent delegation, issue tracking and branch conventions are documented in
`docs/AGENT_WORKFLOW.md`. Each agent:

1. Opens a workstream-claim issue from `.github/ISSUE_TEMPLATE/workstream-claim.md`.
2. Creates branch `wstream/<lane>/<issue>-<slug>` from `master`.
3. Works only inside the lane's owned paths (`.AGENTS/WORKSTREAMS.md`).
4. Runs change-scoped validators and records results.
5. Opens a PR using the handoff template below; the coordinator reviews.
6. The PR updates `.AGENTS/TRACKER.md` (status) and `.AGENTS/EVIDENCE.md` (evidence).

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
