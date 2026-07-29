# Active delivery tracker

## Baseline

| Field | Value |
|---|---|
| Commit | `63c2407be182c1ff78a2806f84bdf90b3c86f4e3` |
| Branch | `master` |
| Dirty baseline | Clean |
| Delivery | Integrated 0.2.13 security cycle |
| Coordinator | Codex |

## Board

| ID | Owner | Status | Evidence |
|---|---|---|---|
| STG | Coordinator | done | elevated upgrade protected ProgramData; standard token cannot inspect/traverse installer root |
| IPC | Coordinator | done | stalled client left concurrent status at 25 ms and was disconnected after deadline |
| SHD | Coordinator | done | stop two seconds into reconciliation returned 0 and recovered the same READY controller |
| SUP | Coordinator | blocked | RustSec and upstream Authenticode pass; trusted product certificate absent |
| REL | Coordinator | blocked | upgrade, repair and hostile lifecycle acceptance pass; signed production build pending |

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
