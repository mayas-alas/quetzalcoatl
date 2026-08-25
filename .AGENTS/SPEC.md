# 0.2 platform — additional services workstream specification

## New services to add

| Service | Repo | Instances | VMID range | LXC name pattern | Tailscale tag |
|---|---|---|---|---|---|
| FreeLLMAPI | github.com/tashfeenahmed/freellmapi | 1 | 300 | gnx-freellmapi-1 | tag:quetzalcoatl-freellmapi |
| OmniRoute | github.com/diegosouzapw/OmniRoute | 1 | 302 | gnx-omniroute-1 | tag:quetzalcoatl-omniroute |

## Acceptance criteria

- Each instance is a separate LXC created via OpenTofu service root
- Each LXC runs Docker with Compose (Tailscale sidecar + app container)
- Tailscale enrollment uses `tag:quetzalcoatl-<service>` (not tag:quetzalcoatl-service)
- HTTPS exposed via Tailscale serve (port 443 on tailscale0, no public exposure)
- No mutable image tags — all images pinned by digest
- No PVE credentials enter Forgejo, registry, Actions, or runner
- No Tailscale enrollment credentials in repositories, Compose, logs, argv, OpenTofu state
- installer/build.ps1 remains the release entry point

## Contract invariants preserved

- Runtime generation stays `proxmox-platform`; payload contract stays `6`
- Platform state remains separate from core controller/member schema
- Closed remote argv; variable data uses bounded stdin
- PVE readiness precedes platform reconciliation
- No localhost UI, Windows listener, new Windows product port, or Tauri runtime
- Images use immutable digests; mutable tags prohibited

## Implementation lanes

The deployment topology (instance count, VMID base, Tailscale tag, port, health
path) is Platform-bundle policy, closed in `platform/services/<slug>/policy.json`
and enforced by `platform/operations/deploy`; the service repository only
publishes the OCI digest and its bounded port/health declaration (`schema: 2`).
Deployment creates one LXC per service (`gnx-<slug>-1`, one OpenTofu state key)
from the single `tofu/service/main.tf` root. This QA topology is superseded by
Issue #17; convergence must not destroy legacy VMIDs 301 or 303 if they exist.

### Agent B (platform runtime) — owns all changes

| Path | Change |
|---|---|
| `platform/tofu/service/` | Keep only the single `main.tf` service root; widen VMID range and hostname pattern; delete the counted `freellmapi.tf`/`oninroute.tf` copies (prohibited parallel templates) |
| `platform/services/{freellmapi,omniroute}/` | `compose.yml` + `serve.json` (per-service port, health, tag) plus a locked singleton `policy.json` (`instances: 1`, `vm_id_base`, `tag`) |
| `platform/operations/deploy` | Per-instance loop driven by the policy file; per-instance state key and hostname; bounded health probe |
| `platform/operations/lxc-service` | `service` kind accepts the per-service tag and `gnx-*` hostname |
| `platform/operations/{discover-releases,verify-release}.py` | Schema 2 with bounded port/health-path |
| `platform/manifest.toml` | Keep runner/image digests |
| `platform/platform.lock.json` | Regenerate with added and removed files |
| `runtime/` | No changes (Tailscale sidecar reused as-is) |

### Agent C (delivery assurance) — validator updates

| Path | Change |
|---|---|
| `tools/validation/platform.py` | Assert new service directories exist and have required files |
| `tools/validation/repository.py` | Add expected service paths to inventory |

### Coordinator — cross-lane integration

| Path | Change |
|---|---|
| `.AGENTS/TRACKER.md` | Add new workstream board entries |
| `.AGENTS/SCOPE.md` | Extend included outcomes if needed |
| `CHANGELOG.md` | Record feature addition |

## Handoff template for Agent B

```text
Workstream: freellmapi-omniroute
Changed paths: platform/tofu/service/*.tf, platform/services/freellmapi/*, platform/services/omniroute/*, platform/manifest.toml, platform/platform.lock.json
Contract impact: New service slugs (freellmapi, omniroute), new Tailscale tags, 2 managed LXC VMIDs (300 and 302). VMIDs 301 and 303 are never destroyed automatically. No Rust contract changes.
Checks: platform.lock.json parses; platform.py validator passes; compose files use only immutable digests; Tailscale tags match spec.
Known failures: None expected.
Next dependency: Agent C platform.py validation update; then Coordinator evidence recording.
```

## Reference materials

- FreeLLMAPI: https://github.com/tashfeenahmed/freellmapi (OpenAI-compatible `/v1/messages` and `/v1/embeddings`)
- OmniRoute: https://github.com/diegosouzapw/OmniRoute (multi-provider LLM router)
- Existing Forgejo service pattern in `platform/services/forgejo/` is the canonical template
