# Active delivery tracker

## Baseline

| Field | Value |
|---|---|
<<<<<<< HEAD
| Commit | `9f07372e14077c7dc909041c488580e06f71a923` |
| Branch | `master` |
| Installed product | 0.2.41 QA-chain-signed |
| Physical state | controller service and platform READY; authorized QA Smart App Control Off transition applied after restart; CLI and tray recovered |
| Candidate | 0.2.42 |
| Coordinator | Codex (closing lane claimed 2026-08-23: QA-signed 0.2.42 build, per-agent commit history, `mayas-alas/quetzalcoatl` GitHub publication, release zipped without private keys) |
=======
| Commit | `3df17c4344169b01dc0cd6a68820c22cb7ad2802` |
| Branch | `master` |
| Installed product | 0.2.41 QA-chain-signed |
| Physical state | controller service and platform READY; authorized QA Smart App Control Off transition applied after restart; CLI and tray recovered |
| Candidate | 0.2.41 |
| Coordinator | Codex |
>>>>>>> origin/master

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
<<<<<<< HEAD
| PKG | review | official 0.2.41 QA build, MSI/Burn extraction and production-without-QA-payload probe pass; 0.2.42 unsigned development build passed `tools/check.ps1 -SourceOnly` and produced coherent MSI + Setup (SHA-256 in EVIDENCE); public release signing remains blocked |
| PHY | review | 0.2.40 to 0.2.41 upgrade and 0.2.41 repair exited 0; controller and platform returned READY with one visible/one hidden registration; physical 0.2.42 install/upgrade/repair not yet executed |
| SIG | review | ten-year QA root, renewable publisher and installer-driven machine trust passed physically; 0.2.42 build adopted the closed `CN=GNX Labs QA Publisher` subject (the malformed `CN=GNX Labs. QA Publisher` was corrected in source and signing); production still requires Windows AuthRoot |
| PUB | active | coordinator closing lane claimed 2026-08-23; awaiting Agent C `-QaSigning` build, per-agent commit history, `mayas-alas/quetzalcoatl` repo creation and `v0.2.42-qa` release upload |
=======
| PKG | review | official 0.2.41 QA build, MSI/Burn extraction and production-without-QA-payload probe pass; public release signing remains blocked |
| PHY | review | 0.2.40 to 0.2.41 upgrade and 0.2.41 repair exited 0; controller and platform returned READY with one visible/one hidden registration |
| SIG | review | ten-year QA root, renewable publisher and installer-driven machine trust passed physically; production still requires Windows AuthRoot |
>>>>>>> origin/master

## Active blockers

| ID | Finding | Required resolution |
|---|---|---|
| B40-1 | Smart App Control Enforce blocks the self-signed payload: 0.2.27 was rejected before QA changed the policy from `1` to `0`; the apparent 0.2.33-0.2.38 success occurred while it was Off, and restored Enforce rejects 0.2.39/0.2.40. | Keep local signing for controlled QA, but obtain trusted signing for physical release acceptance; do not silently mutate the host policy. |
<<<<<<< HEAD
| PUB-1 | No GitHub remote exists; repo `mayas-alas/quetzalcoatl` must be created; QA-signed 0.2.42 build must complete before release zips are assembled; per-agent commits required before push. | Delegate Agent C `-QaSigning` build; after success, coordinator performs per-agent commits, repo creation, push, and release `v0.2.42-qa` with minimal installer zip (no private keys). |
| PUB-2 | Three pre-existing source-gate blockers prevent `tools/check.ps1 -SourceOnly` green: stale version validator (`contracts.py`), missing `TROUBLESHOOTING.md` in repository taxonomy, environment-dependent Docker test. | Coordinator records these as known; they are not mutated out of scope. The QA signing build lane is orthogonal and must proceed. |

## Recent progress

- **Initial setup** (`1cff453`): Created `.AGENTS` tracking files (README.md, SCORE.md, WORKSTREAMS.md, TRACKER.md) to manage the 0.2 platform foundation stabilization workstream.
- **Embeddings API**: Attempted to query `/v1/embeddings` with query "quetzalcoatl" and model "auto". The endpoint responded with a server error indicating no usable embedding keys are configured. This is expected in the development environment without proper embedding provider keys.
- **Git**: Initial commit established the .AGENTS directory structure. Ready for further commits.
- **Coordinator closing lane claimed** (2026-08-23): Assigned Agent C to produce `-QaSigning` 0.2.42 build; will assemble minimal installer zip (Setup + MSI + platform lock files + SHA256SUMS, no private keys), perform per-agent commit history, create `mayas-alas/quetzalcoatl` repo, push, and publish `v0.2.42-qa` release.

## Next steps

1. Complete the Smart App Control remediation (B40-1) by obtaining trusted signing for the QA payload.
2. Finalize the platform foundation stabilization (0.2.41) and prepare for release.
3. Ensure all blockers are resolved before merging to production.
4. Agent C: run `installer/build.ps1 -QaSigning` to produce QA-signed 0.2.42 artifacts (Setup, MSI, payload locks).
5. Coordinator: assemble minimal release zip (target/installer/QuetzalcoatlSetup.exe, Quetzalcoatl.msi, platform-payload/*, runtime-payload/payload.lock.json, SHA256SUMS).
6. Coordinator: per-agent commits (Agent B rename fix, Agent A contract/version, Agent C installer/signing).
7. Coordinator: create `mayas-alas/quetzalcoatl` GitHub repo, push master, tag `v0.2.42-qa`, upload zip as release asset.
8. Record final evidence in EVIDENCE.md and TRACKER.md.

## Notes

- Domain: email.gnx
- Email: [EMAIL]
- Agent name: maya
- Base URL: http://localhost:31415/v1
- API Key: freellmapi-50698af42b84ff91b4313e372da172138f6fb1b188810bc6
=======

## Resolved blockers

| ID | Resolution |
|---|---|
| B40-2 | Advanced the material installer change to 0.2.41 with fresh ProductCode, PackageCode and BundleId; upgrade from cached 0.2.40 passed. |
| B40-3 | Authorized restart applied Smart App Control Off; CLI and tray recovered before the 0.2.41 work. |
>>>>>>> origin/master
