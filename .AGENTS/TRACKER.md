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
| FRE | blocked | source definitions committed and validators pass, but the spec (VMIDs 300-303, `gnx-freellmapi-{1,2}` names, 2 instances per service, per-service Tailscale tags/ports) is incompatible with the locked SVC contract; the committed templates are not wired into the runtime deployment path (see FRE-2) |
| RTM | done | `runtime.py` validator now passes: corrected `gnx-tailscale-rename` SHA-256 in `runtime/payload.lock.json` to `b9fce7fe...` matching on-disk command file |

## Active blockers

| ID | Finding | Required resolution |
|---|---|---|
| B40-1 | Smart App Control Enforce blocks the self-signed payload: 0.2.27 was rejected before QA changed the policy from `1` to `0`; the apparent 0.2.33-0.2.38 success occurred while it was Off, and restored Enforce rejects 0.2.39/0.2.40. | Keep local signing for controlled QA, but obtain trusted signing for physical release acceptance; do not silently mutate the host policy. |
| PHY-1 | Physical execution of 0.2.42 (install/upgrade/repair on this host) and the 4 new FreeLLMAPI/OmniRoute LXCs has not yet been exercised. | Coordinate elevated Setup run on the Proxmox controller; deploy the new services via the closed SVC path; verify Tailscale HTTPS access and health probes. |
| FRE-2 | Spec/contract gap: the locked bundle deploys services only through `platform/operations/deploy` + `discover-releases.py`/`verify-release.py` + `tofu/service/main.tf` + `services/service/compose.yml` + `lxc-service`. That path enforces VMID 1000-7999, hostname `gnx-svc-<slug>`, one LXC per source repo, port 8080, health `/` and Tailscale tag `tag:quetzalcoatl-service`. The committed `tofu/service/{freellmapi,omniroute}.tf` (count=2, vm_id_start=300) and `services/{freellmapi,omniroute}/*` (ports 3001/20128, `/healthz`, per-service tags) are never referenced by any runtime operation — they are parallel transitional templates, which SCOPE prohibits. `platform.py`/`repository.py` assert file presence only, not wiring. | Explicit scope amendment required: either (a) extend the SVC contract to per-service templates (multi-instance, custom VMID range, per-service tags/ports) with validator + regression coverage, or (b) drop the spec deltas and deploy both services through the closed single-instance path (one LXC each, hash-derived VMID, generic compose, `tag:quetzalcoatl-service`) after OCI-1 publishes their images. |
| SEC-1 | Plaintext FreeLLMAPI API key found in historical commit `edc28d4` inside `.AGENTS/TRACKER.md`; current tree is clean. Public history cannot be rewritten; forward-fix is redaction. Token rotation is the service owner's responsibility. |
| OCI-1 | FreeLLMAPI/OmniRoute image digests are pinned in manifest/compose but no image has been built/published by the OCI runner lane. | Agent A / OCI lane: build both images via the Forgejo template + dedicated runner and publish by digest; then reconcile digests. |

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

## Next steps

1. Complete the Smart App Control remediation (B40-1) by obtaining trusted signing for the QA payload.
2. **Coordinator**: physical execution of 0.2.42 install/upgrade/repair on the Proxmox controller, and deployment of 4 new FreeLLMAPI/OmniRoute LXCs via OpenTofu.
3. **Agent A / OCI lane**: build and publish FreeLLMAPI and OmniRoute OCI images by digest via the Forgejo template + dedicated runner.
4. Verify Tailscale HTTPS access and health probes for all 4 new services.

## Notes

- Domain: email.gnx