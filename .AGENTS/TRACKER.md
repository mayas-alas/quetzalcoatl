# Active delivery tracker

## Baseline

| Field | Value |
|---|---|
| Commit | `d84257a77ed74b5f4ea0951aca82083ff6ab3483` |
| Branch | `master` |
| Dirty baseline | Existing uncommitted 0.2.0 migration explicitly retained |
| Delivery | One integrated 0.2.1 installer-recovery cycle |
| Coordinator | Codex |

## Board

| ID | Owner | Status | Depends on | Next gate |
|---|---|---|---|---|
| LEG | Agent A | done | baseline | AGPL, notices and packaged legal inventory pass |
| BRD | Agent C | done | LEG | canonical G icon and 165x312 Burn derivative pass |
| TYP | Agent A/B | done | ARC boundary | closed health/stage/PVE URL tests pass |
| ARC | Agent B | done | baseline | real modules compile without path wiring or production globs |
| RUN | Agent B | done | ARC | installed payload and embedded operations are disjoint |
| REL | Agent A/C | done | REC | 0.2.1 identities and artifact hashes pass |
| DOC | Agent C | done | final taxonomy | five-file documentation inventory passes |
| DEL | Agent C | review | REC | MSI extraction and deterministic Setup build pass |
| REC | Agent C | active | 0.2.0 incident | physical recovery is waiting on the required Windows restart |
| INT | Coordinator | review | all rows | integrated gate passed; physical resume pending |

## Blockers

| ID | Dependency | Owner | Status |
|---|---|---|---|
| B01 | Choose AGPL expression: this cycle uses `AGPL-3.0-only` from the user's “AGPL 3.0” instruction. | Coordinator | resolved |
| B02 | Hosted Windows CI is excluded by user instruction. | Coordinator | resolved |
| B03 | 0.2.0 reused ProductCode/PackageCode across different MSI bytes; Windows Installer repaired from stale inventory and left runtime empty. | Coordinator | resolved by 0.2.1 scope |

## Handoffs

| From | To | Requirement | Status |
|---|---|---|---|
| A | C | Product license and copyright must be packaged by MSI/Burn. | done |
| B | C | Installer must stage the final runtime taxonomy and lock exactly. | done |
| A/B/C | Coordinator | One source/build gate and artifact inventory. | done |

## Residual acceptance

Hosted Windows CI remains intentionally excluded. The current host contains the
reproducible broken 0.2.0 runtime and is the required 0.2.1 recovery fixture. The
0.2.1 bundle is cached and registered `Reboot Pending`; its MSI has not run yet.
