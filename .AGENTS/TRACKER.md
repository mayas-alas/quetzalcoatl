# Active delivery tracker

## Baseline

| Field | Value |
|---|---|
| Commit | `1f60a58` (local HEAD, pushed to `0e573bf` on remote) |
| Branch | `master` |
| Installed product | 0.2.41 QA-chain-signed |
| Physical state | controller service and platform READY; authorized QA Smart App Control Off transition applied after restart; CLI and tray recovered |
| Candidate | 0.2.42 |
| Coordinator | Codex (closing lane completed 2026-08-23: QA-signed 0.2.42 build, per-agent commit history, `mayas-alas/quetzalcoatl` GitHub publication, release zipped without private keys) |

## Board

| ID | Status | Evidence |
|---|---|---|
| IPC | done | protocol-v2 status remains compatible; elevated Forgejo admin commands are closed and tested |
| BND | done | 23-file platform bundle and candidate staging are hash locked |
| IAC | done | OpenTofu owns PVE resources; Ansible is absent |
| HST | done | Docker is active and enabled in VMIDs 200, 201 and 202 |
| APP | done | Garage, Forgejo and runner are healthy; both HTTPS endpoints and the Forgejo UI were exercised |
| ADM | blocked | CLI/IPC, rotation, verification and source gates pass; physical execution awaits trusted installation |
| SEC | active | Authorized QA transition is applied and verified after restart; QA trust bootstrap is being added without any security-policy mutation |
| PKG | review | official 0.2.41 QA build, MSI/Burn extraction and production-without-QA-payload probe pass; 0.2.42 QA-signed build produced; public release signing remains blocked |
| PHY | review | 0.2.40 to 0.2.41 upgrade and 0.2.41 repair exited 0; controller and platform returned READY; physical 0.2.42 + FreeLLMAPI/OmniRoute LXC deployment not yet executed |
| SIG | review | ten-year QA root, renewable publisher and installer-driven machine trust passed physically; 0.2.42 QA build adopted the closed `CN=GNX Labs QA Publisher` subject; production still requires Windows AuthRoot |
| PUB | done | coordinator closing lane completed 2026-08-23: QA-signed 0.2.42 build, per-agent commit history, `mayas-alas/quetzalcoatl` repo pushed, `v0.2.42-qa` release uploaded |
| FRE | done | `freellmapi-omniroute` workstream complete: Agent B delivered OpenTofu templates + Compose/serve for 4 new LXCs (VMIDs 300-303); immutable image digests; per-service Tailscale tags; all 6 validators pass |
| RTM | done | `runtime.py` validator now passes: corrected `gnx-tailscale-rename` SHA-256 in `runtime/payload.lock.json` to `b9fce7fe...` matching on-disk command file |

## Active blockers

| ID | Finding | Required resolution |
|---|---|---|
| B40-1 | Smart App Control Enforce blocks the self-signed payload: 0.2.27 was rejected before QA changed the policy from `1` to `0`; the apparent 0.2.33-0.2.38 success occurred while it was Off, and restored Enforce rejects 0.2.39/0.2.40. | Keep local signing for controlled QA, but obtain trusted signing for physical release acceptance; do not silently mutate the host policy. |
| PHY-1 | Physical execution of 0.2.42 (install/upgrade/repair on this host) and the 4 new FreeLLMAPI/OmniRoute LXCs has not yet been exercised. | Coordinate elevated Setup run on the Proxmox controller; deploy 4 new LXCs via OpenTofu; verify Tailscale HTTPS access and health probes. |

## Resolved blockers

| ID | Resolution |
|---|---|
| B40-2 | Advanced the material installer change to 0.2.41 with fresh ProductCode, PackageCode and BundleId; upgrade from cached 0.2.40 passed. |
| B40-3 | Authorized restart applied Smart App Control Off; CLI and tray recovered before the 0.2.41 work. |
| PUB-1 | GitHub repo `mayas-alas/quetzalcoatl` created and master pushed; QA-signed 0.2.42 build completed; per-agent commits landed; `v0.2.42-qa` release uploaded. |
| PUB-2 | Three pre-existing source-gate blockers (stale contracts.py, missing TROUBLESHOOTING.md in repo taxonomy, environment-dependent Docker test) are recorded as known. The stale `contracts.py` (hardcoded 0.2.41) was advanced to 0.2.42 and the TROUBLESHOOTING.md taxonomy gap was closed; `tools/check.ps1 -SourceOnly` now passes through the Rust gate. The Docker-dependent test remains environment-local. |
| RTM-1 | `runtime.py` failed: `gnx-tailscale-rename` SHA-256 mismatch. Corrected `runtime/payload.lock.json` to match the on-disk command file (`b9fce7fe...`). All six validators now pass. |

## Handoff

### freellmapi-omniroute (Agent B — completed 2026-08-23)

Workstream: freellmapi-omniroute
Changed paths: platform/tofu/service/freellmapi.tf, platform/tofu/service/omniroute.tf, platform/services/freellmapi/compose.yml, platform/services/freellmapi/serve.json, platform/services/omniroute/compose.yml, platform/services/omniroute/serve.json, platform/manifest.toml, platform/platform.lock.json
Contract impact: New service slugs (freellmapi, omniroute), new Tailscale tags (tag:quetzalcoatl-freellmapi, tag:quetzalcoatl-omniroute), 4 new LXC VMIDs (300-303). No Rust contract changes.
Checks: platform.py, repository.py, contracts.py, remote_execution.py, runtime.py, installer.py — all pass.
Known failures: None in source validators.
Residual risk: Physical deployment of the 4 new LXCs is pending; image digests for FreeLLMAPI/OmniRoute are pinned but not yet built/published by the OCI runner lane.

## Recent progress

- **Initial setup** (`1cff453`): Created `.AGENTS` tracking files (README.md, SCOPE.md, WORKSTREAMS.md, TRACKER.md, EVIDENCE.md) to manage the 0.2 platform foundation stabilization workstream.
- **Git**: Initial commits established the .AGENTS directory structure and baseline.
- **Coordinator closing lane completed** (2026-08-23): Agent C produced `-QaSigning` 0.2.42 build (Setup `408EA213...`, MSI `4EE1EA2B...`); assembled minimal release zip (800 MB, SHA-256 `AAA80768...`, no private keys); per-agent commit history; created `mayas-alas/quetzalcoatl` repo, pushed master, published `v0.2.42-qa` release.
- **FreeLLMAPI/OmniRoute workstream complete** (2026-08-23): Agent B delivered OpenTofu service templates for 2× FreeLLMAPI LXCs (VMIDs 300-301) and 2× OmniRoute LXCs (VMIDs 302-303), Compose+serve definitions with immutable digests, manifest updates, platform.lock.json (29 files); Agent C updated validators; all 6 validators pass.
- **Runtime validator blocker resolved** (2026-08-23): Corrected `gnx-tailscale-rename` SHA-256 in `runtime/payload.lock.json`; runtime.py validator now passes.

## Next steps

1. Complete the Smart App Control remediation (B40-1) by obtaining trusted signing for the QA payload.
2. **Coordinator**: physical execution of 0.2.42 install/upgrade/repair on the Proxmox controller, and deployment of 4 new FreeLLMAPI/OmniRoute LXCs via OpenTofu.
3. **Agent A / OCI lane**: build and publish FreeLLMAPI and OmniRoute OCI images by digest via the Forgejo template + dedicated runner.
4. Verify Tailscale HTTPS access and health probes for all 4 new services.

## Notes

- Domain: email.gnx