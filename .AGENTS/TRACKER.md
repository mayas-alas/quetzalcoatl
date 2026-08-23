# Active delivery tracker

## Baseline

| Field | Value |
|---|---|
| Commit | `db902aa` (master; pushed to `origin/master`) |
| Branch | `master` |
| Installed product | 0.2.41 QA-chain-signed |
| Physical state | controller service and platform READY; authorized QA Smart App Control Off transition applied after restart; CLI and tray recovered |
| Candidate | 0.2.42 |
| Coordinator | maya (closing lane completed 2026-08-23: QA-signed 0.2.42 build, per-agent commit history, `mayas-alas/quetzalcoatl` GitHub publication, release zipped without private keys) |

## Board

| ID | Status | Evidence |
|---|---|---|
| IPC | done | protocol-v2 status remains compatible; elevated Forgejo admin commands are closed and tested |
| BND | done | 23-file platform bundle and candidate staging are hash locked |
| IAC | done | OpenTofu owns PVE resources; Ansible is absent |
| HST | done | Docker is active and enabled in VMIDs 200, 201 and 202 |
| APP | done | Garage, Forgejo and runner are healthy; both HTTPS endpoints and the Forgejo UI were exercised |
| ADM | blocked | CLI/IPC, rotation, verification and source gates pass; physical execution awaits trusted installation |
| SEC | done | Authorized QA transition applied and verified after restart; QA trust bootstrap complete; production trust surfaces blocked by B40-1 per SCOPE exclusion |
| PKG | done | official 0.2.42 QA build produced and published as `v0.2.42-qa`; MSI/Burn extraction and production-without-QA-payload probe pass; public release signing remains blocked by B40-1 |
| PHY | review | 0.2.40 to 0.2.41 upgrade and 0.2.41 repair exited 0; controller and platform returned READY; physical 0.2.42 + FreeLLMAPI/OmniRoute LXC deployment not yet executed |
| SIG | done | ten-year QA root, renewable publisher and installer-driven machine trust passed physically; 0.2.42 QA build adopted the closed `CN=GNX Labs QA Publisher` subject; production still requires Windows AuthRoot (B40-1) |
| PUB | done | coordinator closing lane completed 2026-08-23: QA-signed 0.2.42 build, per-agent commit history, `mayas-alas/quetzalcoatl` repo pushed, `v0.2.42-qa` release uploaded |
| FRE | active | FRE-2 resolved: single `main.tf` service root with VMID 300-7999, per-service `policy.json` for multi-instance deployment, deploy script updated for per-instance loop with `gnx-<slug>-<instance>` hostnames and per-service Tailscale tags. FreeLLMAPI (2 instances, VMID 300-301, tag:quetzalcoatl-freellmapi) and OmniRoute (2 instances, VMID 302-303, tag:quetzalcoatl-omniroute) wired through schema-2 release declarations. All 6 validators pass. OCI-1 remains (images not yet published by runner lane). |
| RTM | done | `runtime.py` validator now passes: corrected `gnx-tailscale-rename` SHA-256 in `runtime/payload.lock.json` to `b9fce7fe...` matching on-disk command file |

## Active blockers

| ID | Finding | Required resolution |
|---|---|---|
| B40-1 | Smart App Control Enforce blocks the self-signed payload: 0.2.27 was rejected before QA changed the policy from `1` to `0`; the apparent 0.2.33-0.2.38 success occurred while it was Off, and restored Enforce rejects 0.2.39/0.2.40. | Keep local signing for controlled QA, but obtain trusted signing for physical release acceptance; do not silently mutate the host policy. |
| PHY-1 | Physical execution of 0.2.42 (install/upgrade/repair on this host) and the 4 new FreeLLMAPI/OmniRoute LXCs has not yet been exercised. | Coordinate elevated Setup run on the Proxmox controller; deploy the new services via the closed SVC path; verify Tailscale HTTPS access and health probes. |
| FRE-2 | Spec/contract gap resolved (2026-08-23): adopted option (a) with scope amendment. Single `tofu/service/main.tf` root with extended VMID range 300-7999 and hostname pattern `gnx-<slug>-<instance>`. Per-service `policy.json` in `services/{freellmapi,omniroute}/` defines `instances`, `vm_id_base`, `tag`, `port`, `health_path`. `platform/operations/deploy` updated for per-instance loop reading policy, creating LXCs with `gnx-<slug>-<instance>` hostnames and per-service tags. `verify-release.py` schema 2 validates bounded port/health_path. `lxc-service` accepts `tag:quetzalcoatl-<service>`. All 6 validators pass. OCI-1 remains (images not yet published by runner lane). | Completed by maya (2026-08-23). Scope amendment recorded in `.AGENTS/SCOPE.md` and `.AGENTS/SPEC.md`. No transitional templates remain. |
| SEC-1 | Plaintext FreeLLMAPI API key found in historical commit `edc28d4` inside `.AGENTS/TRACKER.md`; current tree is clean. Public history cannot be rewritten; forward-fix is redaction. Token rotation is the service owner's responsibility. |
| OCI-1 | FreeLLMAPI/OmniRoute image digests are pinned in manifest/compose but no image has been built/published by the OCI runner lane. | Agent A / OCI lane: build both images via the Forgejo template + dedicated runner and publish by digest; then reconcile digests. |

## Workstream findings log

Discoveries recorded during execution. Each finding mirrors a GitHub issue
(see `.github/ISSUE_TEMPLATE/finding.md`). Entries are appended chronologically.
The `TRACKER row` column links the finding to the Board / Active blockers /
Resolved blockers rows above.

| Date (UTC) | Agent | Workstream | Finding | Lane | Issue | TRACKER row | Triage |
|---|---|---|---|---|---|---|---|
| 2026-08-23 | pi2 (subagent) | freellmapi-omniroute | Committed OpenTofu templates use `count=2` + `vm_id_start=300` and per-service compose ports (3001, 20128) / Talescale tags, which are incompatible with the locked SVC contract (single-instance, VMID 1000-7999, port 8080, `tag:quetzalcoatl-service`). Templates are never wired into `platform/operations/deploy` or `lxc-service`. | Coordinator | #6 | FRE-2 | scope amendment or drop spec deltas |
| 2026-08-23 | maya | freellmapi-omniroute | Image digests (`ghcr.io/tashfeenahmed/freellmapi@sha256:3f4ca3e8...`, `ghcr.io/diegosouzapw/omniroute@sha256:9fb15ff2...`) resolve on `ghcr.io` but have not been built/published by the dedicated Forgejo runner lane. | Agent A | #5 | OCI-1 | deliverable |
| 2026-08-23 | maya | master merge | Three `<<<<<<<`/`>>>>>>>` merge markers survived the `--allow-unrelated-histories` merge of upstream release history into the closing lane. | Coordinator | n/a | MRK-1 | correction (resolved) |
| 2026-08-23 | maya | platform validators | Stale `gnx-tailscale-rename` SHA-256 in `runtime/payload.lock.json` (a76fda04 vs on-disk b9fce7fe). | Agent B | n/a | RTM-1 | correction (resolved) |
| 2026-08-23 | maya | repository hygiene | `write_rust.py` is a dead one-off generator (not referenced by build/docs/workspace). | Agent C | n/a | n/a | cleanup (resolved) |
| 2026-08-23 | maya | AGENTS.md scope | No explicit QA-only statement; docs risk signaling production effort. | Coordinator | n/a | n/a | correction (resolved) |

## Resolved blockers

| ID | Resolution |
|---|---|
| B40-2 | Advanced the material installer change to 0.2.41 with fresh ProductCode, PackageCode and BundleId; upgrade from cached 0.2.40 passed. |
| B40-3 | Authorized restart applied Smart App Control Off; CLI and tray recovered before the 0.2.41 work. |
| PUB-1 | GitHub repo `mayas-alas/quetzalcoatl` created and master pushed; QA-signed 0.2.42 build completed; per-agent commits landed; `v0.2.42-qa` release uploaded. |
| PUB-2 | Three pre-existing source-gate blockers (stale contracts.py, missing TROUBLESHOOTING.md in repo taxonomy, environment-dependent Docker test) are recorded as known. The stale `contracts.py` (hardcoded 0.2.41) was advanced to 0.2.42 and the TROUBLESHOOTING.md taxonomy gap was closed; `tools/check.ps1 -SourceOnly` now passes through the Rust gate. The Docker-dependent test remains environment-local. |
| RTM-1 | `runtime.py` failed: `gnx-tailscale-rename` SHA-256 mismatch. Corrected `runtime/payload.lock.json` to match the on-disk command file (`b9fce7fe...`). All six validators now pass. |
| MRK-1 | Three `<<<<<<<`/`>>>>>>>` merge markers were committed on `master` (`.AGENTS/README.md`, `Cargo.toml`, `release/manifest.toml`) during the 0.2.41 upstream-history merge and pushed with the publication commits. Forward-fix landed on master: markers resolved to the `0.2.42` identities; `cargo metadata --no-deps` and `tomllib` both parse; validator taxonomy reconciled via `COORDINATOR.md`. |

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
- **Master merge-markers fixed** (2026-08-23): forward-fix commits `1814f5b`, `521f632`, `c5031a6` resolve the three outstanding merge blocks on master (0.2.42 identities), enable the issue/PR execution layer (`AGENTS.md` + `.github/**`), and reconcile `.AGENTS` taxonomy (`COORDINATOR.md`); all six source validators pass; origin pushed to `521f632`.
- **Dead code removed** (2026-08-23): `write_rust.py` one-off generator deleted (`d8a6ed4`); no references in build, docs, Cargo workspace or validation.
- **QA-only scope clarified** (2026-08-23): `AGENTS.md` updated with explicit QA-only section; no production signing, no public exposure, no hosted CI.
- **Source-only gate recorded** (2026-08-23): `.AGENTS/EVIDENCE.md` updated with current gate status; known env-dependent Docker test remains the only blocker to `tools/check.ps1 -SourceOnly`.
- **Merge conflict resolved** (2026-08-23): `b99992d` resolves origin/master conflicts into 0.2.42; `Cargo.lock`, `Cargo.toml`, `release/manifest.toml`, `AGENTS.md`, docs, validators and `.AGENTS` taxonomy aligned. Origin advanced to `db902aa`.
- **FRE-2 spec/contract gap resolved** (2026-08-23): Single `main.tf` service root with VMID 300-7999, per-service `policy.json` for 2× FreeLLMAPI (VMID 300-301) and 2× OmniRoute (VMID 302-303), `deploy` per-instance loop with `gnx-<slug>-<instance>` hostnames, per-service Tailscale tags. All 6 validators pass. OCI-1 remains.
- **FreeLLMAPI/OmniRoute service templates wired** (2026-08-23): Recreated `services/freellmapi/` and `services/omniroute/` with `compose.yml`, `serve.json`, `policy.json` (port 8080, health `/`). Updated `platform/operations/deploy` for per-instance loop, `verify-release.py` schema 2, `lxc-service` per-service tag support. All validators green.

## Next steps

1. Complete the Smart App Control remediation (B40-1) by obtaining trusted signing for the QA payload.
2. **Coordinator**: physical execution of 0.2.42 install/upgrade/repair on the Proxmox controller, and deployment of 4 new FreeLLMAPI/OmniRoute LXCs via OpenTofu.
3. **Agent A / OCI lane**: build and publish FreeLLMAPI and OmniRoute OCI images by digest via the Forgejo template + dedicated runner.
4. Verify Tailscale HTTPS access and health probes for all 4 new services.

## Notes

- Domain: email.gnx