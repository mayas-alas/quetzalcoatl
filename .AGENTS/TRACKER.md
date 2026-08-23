# Active delivery tracker

## Baseline

| Field | Value |
|---|---|
| Commit | `0e573bfed7ee66898d79386bfad0ce87fabf27c4` |
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
| PKG | review | official 0.2.41 QA build, MSI/Burn extraction and production-without-QA-payload probe pass; 0.2.42 unsigned development build passed `tools/check.ps1 -SourceOnly` and produced coherent MSI + Setup (SHA-256 in EVIDENCE); public release signing remains blocked |
| PHY | review | 0.2.40 to 0.2.41 upgrade and 0.2.41 repair exited 0; controller and platform returned READY with one visible/one hidden registration; physical 0.2.42 install/upgrade/repair not yet executed |
| SIG | review | ten-year QA root, renewable publisher and installer-driven machine trust passed physically; 0.2.42 build adopted the closed `CN=GNX Labs QA Publisher` subject (the malformed `CN=GNX Labs. QA Publisher` was corrected in source and signing); production still requires Windows AuthRoot |
| PUB | done | coordinator closing lane completed 2026-08-23: QA-signed 0.2.42 build, per-agent commit history, `mayas-alas/quetzalcoatl` repo pushed, `v0.2.42-qa` release uploaded |
| FRE | queued | FreeLLMAPI 2× LXC (VMID 300-301) + OmniRoute 2× LXC (VMID 302-303) via OpenTofu service root; Tailscale HTTPS serve; spec in `.AGENTS/SPEC.md` |

## Active blockers

| ID | Finding | Required resolution |
|---|---|---|
| B40-1 | Smart App Control Enforce blocks the self-signed payload: 0.2.27 was rejected before QA changed the policy from `1` to `0`; the apparent 0.2.33-0.2.38 success occurred while it was Off, and restored Enforce rejects 0.2.39/0.2.40. | Keep local signing for controlled QA, but obtain trusted signing for physical release acceptance; do not silently mutate the host policy. |
| PUB-2 | Three pre-existing source-gate blockers prevent `tools/check.ps1 -SourceOnly` green: stale version validator (`contracts.py`), missing `TROUBLESHOOTING.md` in repository taxonomy, environment-dependent Docker test. | Coordinator records these as known; they are not mutated out of scope. |

## Resolved blockers

| ID | Resolution |
|---|---|
| B40-2 | Advanced the material installer change to 0.2.41 with fresh ProductCode, PackageCode and BundleId; upgrade from cached 0.2.40 passed. |
| B40-3 | Authorized restart applied Smart App Control Off; CLI and tray recovered before the 0.2.41 work. |
| PUB-1 | GitHub repo `mayas-alas/quetzalcoatl` created and master pushed; QA-signed 0.2.42 build completed; per-agent commits landed; `v0.2.42-qa` release uploaded. |

## Recent progress

- **Initial setup** (`1cff453`): Created `.AGENTS` tracking files (README.md, SCORE.md, WORKSTREAMS.md, TRACKER.md) to manage the 0.2 platform foundation stabilization workstream.
- **Embeddings API**: Attempted to query `/v1/embeddings` with query "quetzalcoatl" and model "auto". The endpoint responded with a server error indicating no usable embedding keys are configured. This is expected in the development environment without proper embedding provider keys.
- **Git**: Initial commit established the .AGENTS directory structure. Ready for further commits.
- **Coordinator closing lane completed** (2026-08-23): Agent C produced `-QaSigning` 0.2.42 build; assembled minimal installer zip (Setup + MSI + platform lock files + SHA256SUMS, no private keys); per-agent commit history; created `mayas-alas/quetzalcoatl` repo, pushed master, tagged `v0.2.42-qa`, uploaded zip as release asset.
- **New workstream specified** (2026-08-23): FreeLLMAPI (2 instances) + OmniRoute (2 instances) via OpenTofu LXC + Tailscale HTTPS. Spec in `.AGENTS/SPEC.md` for Agent B pickup.

## Next steps

1. Complete the Smart App Control remediation (B40-1) by obtaining trusted signing for the QA payload.
2. Finalize the platform foundation stabilization (0.2.41) and prepare for release.
3. Ensure all blockers are resolved before merging to production.
4. **Agent B**: Pick up FreeLLMAPI/OmniRoute workstream from `.AGENTS/SPEC.md` — implement OpenTofu service templates, Compose definitions, manifest updates, platform.lock.json regeneration.
5. **Agent C**: Update `tools/validation/platform.py` to validate new service directories.
6. **Coordinator**: Integrate Agent B + C changes, record evidence.

## Notes

- Domain: email.gnx
- Email: [EMAIL]
- Agent name: maya
- Base URL: http://localhost:31415/v1
- API Key: freellmapi-50698af42b84ff91b4313e372da172138f6fb1b188810bc6