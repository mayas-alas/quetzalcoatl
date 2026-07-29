# Active delivery tracker

## Baseline

| Field | Value |
|---|---|
| Commit | `d84257a77ed74b5f4ea0951aca82083ff6ab3483` |
| Branch | `master` |
| Dirty baseline | Existing uncommitted MVP migration retained |
| Delivery | Integrated 0.2.12 maintenance cycle |
| Coordinator | Codex |

## Board

| ID | Owner | Status | Evidence |
|---|---|---|---|
| LEG | Agent A | done | license, notice and packaged legal inventory pass |
| BRD | Agent C | done | one installer assets tree and G branding pass |
| ARC | Agent A/B | done | four packages and one semantic implementation |
| RUN | Agent B | done | closed argv/stdin and managed VM lifecycle pass |
| REC | Agent B/C | done | payload validation and preserved-state reinstall reach READY |
| RBT | Agent B/C | done | repair stop/restart returns 0 and reaches READY |
| ARP | Agent C | done | one visible bundle plus one hidden internal MSI |
| UNS | Agent B/C | done | physical uninstall leaves zero product residue |
| REL | Coordinator | done | 0.2.12 gate, identities and hashes recorded |
| INT | Coordinator | done | uninstall, reinstall and repair physically accepted |

## Resolved blockers

| ID | Finding | Resolution |
|---|---|---|
| B08 | Tray close originally ran after file removal. | CloseApplications is sequenced before `RemoveFiles`. |
| B09 | Burn cleanup could not remove the locked empty root. | Transitional cleanup package and helper were deleted. |
| B10 | Old Podman WSL processes inherited the product root as CWD. | WinSW stop runs a closed managed-machine stop under the service identity. |
| B11 | WinSW waited for the main service after the stop helper returned. | A service-private event terminates the main process after VM stop. |
| B12 | Hosted Windows CI is unavailable. | Explicitly excluded; local integrated gate remains authoritative. |

## Residual acceptance

Direct upgrade from 0.1.17 and manual visual inspection of the tray menu remain
outside this uninstall cycle. No known residual risk blocks 0.2.12 maintenance.
