# 0.2 platform — additional services workstream specification

## New services to add

| Service | Repo | Instances | VMID range | LXC name pattern | Tailscale tag |
|---|---|---|---|---|---|
| FreeLLMAPI | github.com/tashfeenahmed/freellmapi | 2 | 300-301 | gnx-freellmapi-{1,2} | tag:quetzalcoatl-freellmapi |
| OmniRoute | github.com/diegosouzapw/OmniRoute | 2 | 302-303 | gnx-omniroute-{1,2} | tag:quetzalcoatl-omniroute |

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

### Agent B (platform runtime) — owns all changes

| Path | Change |
|---|---|
| `platform/tofu/service/` | Add `freellmapi.tf` and `omniroute.tf` (or parameterized single template) for 2× LXC each |
| `platform/services/freellmapi/` | Compose.yml + serve.json for FreeLLMAPI (2 instances → 2 LXCs, each with single Compose) |
| `platform/services/omniroute/` | Compose.yml + serve.json for OmniRoute (2 instances → 2 LXCs) |
| `platform/manifest.toml` | Add image digests for both services |
| `platform/platform.lock.json` | Update with new files + SHAs |
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
Contract impact: New service slugs (freellmapi, omniroute), new Tailscale tags, 4 new LXC VMIDs (300-303). No Rust contract changes.
Checks: platform.lock.json parses; platform.py validator passes; compose files use only immutable digests; Tailscale tags match spec.
Known failures: None expected.
Next dependency: Agent C platform.py validation update; then Coordinator evidence recording.
```

## Reference materials

- FreeLLMAPI: https://github.com/tashfeenahmed/freellmapi (OpenAI-compatible `/v1/messages` and `/v1/embeddings`)
- OmniRoute: https://github.com/diegosouzapw/OmniRoute (multi-provider LLM router)
- Existing Forgejo service pattern in `platform/services/forgejo/` is the canonical template