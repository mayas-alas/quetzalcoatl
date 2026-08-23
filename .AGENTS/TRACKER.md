# Active delivery tracker

## Baseline

| Field | Value |
|---|---|
| Commit | `3df17c4344169b01dc0cd6a68820c22cb7ad2802` |
| Branch | `master` |
| Installed product | 0.2.41 QA-chain-signed |
| Physical state | controller service and platform READY; authorized QA Smart App Control Off transition applied after restart; CLI and tray recovered |
| Candidate | 0.2.41 |
| Coordinator | Codex |

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
| PKG | review | official 0.2.41 QA build, MSI/Burn extraction and production-without-QA-payload probe pass; public release signing remains blocked |
| PHY | review | 0.2.40 to 0.2.41 upgrade and 0.2.41 repair exited 0; controller and platform returned READY with one visible/one hidden registration |
| SIG | review | ten-year QA root, renewable publisher and installer-driven machine trust passed physically; production still requires Windows AuthRoot |

## Active blockers

| ID | Finding | Required resolution |
|---|---|---|
| B40-1 | Smart App Control Enforce blocks the self-signed payload: 0.2.27 was rejected before QA changed the policy from `1` to `0`; the apparent 0.2.33-0.2.38 success occurred while it was Off, and restored Enforce rejects 0.2.39/0.2.40. | Keep local signing for controlled QA, but obtain trusted signing for physical release acceptance; do not silently mutate the host policy. |

## Resolved blockers

| ID | Resolution |
|---|---|
| B40-2 | Advanced the material installer change to 0.2.41 with fresh ProductCode, PackageCode and BundleId; upgrade from cached 0.2.40 passed. |
| B40-3 | Authorized restart applied Smart App Control Off; CLI and tray recovered before the 0.2.41 work. |
