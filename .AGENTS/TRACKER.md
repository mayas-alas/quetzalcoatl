# Active delivery tracker

## Baseline

| Field | Value |
|---|---|
| Commit | `63c2407be182c1ff78a2806f84bdf90b3c86f4e3` |
| Branch | `master` |
| Dirty baseline | Clean |
| Delivery | Integrated 0.2.14 legal identity cycle |
| Coordinator | Codex |

## Board

| ID | Owner | Status | Evidence |
|---|---|---|---|
| STG | Coordinator | done | elevated upgrade protected ProgramData; standard token cannot inspect/traverse installer root |
| IPC | Coordinator | done | stalled client left concurrent status at 25 ms and was disconnected after deadline |
| SHD | Coordinator | done | stop two seconds into reconciliation returned 0 and recovered the same READY controller |
| LEG | Coordinator | done | GNX Labs manufacturer and joint GNX Labs/Hector AB copyright verified across all product PE files and Setup |
| LNK | Coordinator | done | initial Setup page visibly exposes both links through the canonical repository LICENSE control |
| SUP | Coordinator | blocked | RustSec passes; self-signed/timestamped QA path passes and is rejected by production mode; publicly trusted certificate absent |
| REL | Coordinator | blocked | 0.2.14 source/build, exact-MSI upgrade state, repair, uninstall and fresh-install lifecycle pass; publicly trusted production build remains |

## Resolved blockers

| ID | Finding | Resolution |
|---|---|---|
| B08 | Tray close originally ran after file removal. | CloseApplications is sequenced before `RemoveFiles`. |
| B09 | Burn cleanup could not remove the locked empty root. | Transitional cleanup package and helper were deleted. |
| B10 | Old Podman WSL processes inherited the product root as CWD. | WinSW stop runs a closed managed-machine stop under the service identity. |
| B11 | WinSW waited for the main service after the stop helper returned. | A service-private event terminates the main process after VM stop. |
| B12 | Hosted Windows CI is unavailable. | Explicitly excluded; local integrated gate remains authoritative. |

## Active blockers

| ID | Finding | Required resolution |
|---|---|---|
| B13 | No trusted code-signing certificate is installed on the build host. | Provision a production certificate; the release gate must reject unsigned output. |
