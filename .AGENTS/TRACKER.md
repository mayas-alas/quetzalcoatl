# Active delivery tracker

## Baseline

| Field | Value |
|---|---|
| Commit | `3df17c4344169b01dc0cd6a68820c22cb7ad2802` |
| Branch | `master` |
| Installed product | 0.2.39 development-signed QA |
| Physical state | controller service and platform running; QA CLI blocked by Smart App Control |
| Candidate | 0.2.40 |
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
| SEC | active | User authorized controlled QA transition to Off; registry is `0`, while the already-loaded Enforce policy still rejects binaries until Windows restarts |
| PKG | blocked | signed 0.2.40 QA artifact passed coherence; Smart App Control rejects its self-signed identity |
| PHY | blocked | 0.2.39 platform passed; 0.2.40 show/reset requires a trusted upgrade artifact |
| SIG | active | Closed signature/version inventory passes; authorized local-QA continuation is waiting for the restart required to unload Smart App Control Enforce |

## Active blockers

| ID | Finding | Required resolution |
|---|---|---|
| B40-1 | Smart App Control Enforce blocks the self-signed payload: 0.2.27 was rejected before QA changed the policy from `1` to `0`; the apparent 0.2.33-0.2.38 success occurred while it was Off, and restored Enforce rejects 0.2.39/0.2.40. | Keep local signing for controlled QA, but obtain trusted signing for physical release acceptance; do not silently mutate the host policy. |
| B40-2 | Rebuilding installed 0.2.40 changes timestamped package bytes, while the current ProductCode/PackageCode/BundleId are already cached on the host. | Provision an RSA code-signing identity chaining to Windows AuthRoot, assign fresh MSI/Burn identities, then build and exercise upgrade, repair, CLI and tray under Smart App Control Enforce. |
| B40-3 | `VerifiedAndReputablePolicyState` is now `0`, but policy `{0283ac0f-fff1-49ae-ada1-8a933130cad6}` remains loaded and blocked fresh CLI/tray probes. | Restart Windows, verify CLI/tray and continue the explicitly authorized local-QA path. |
