# Active delivery tracker

## Baseline

| Field | Value |
|---|---|
| Commit | `d84257a77ed74b5f4ea0951aca82083ff6ab3483` |
| Branch | `master` |
| Dirty baseline | Existing uncommitted 0.2.0 migration explicitly retained |
| Delivery | One integrated 0.2.0 hardening cycle |
| Coordinator | Codex |

## Board

| ID | Owner | Status | Depends on | Next gate |
|---|---|---|---|---|
| LEG | Agent A | done | baseline | AGPL, notices and packaged legal inventory pass |
| BRD | Agent A | done | LEG | canonical G icon and 165x312 Burn derivative pass |
| TYP | Agent A/B | done | ARC boundary | closed health/stage/PVE URL tests pass |
| ARC | Agent B | done | baseline | real modules compile without path wiring or production globs |
| RUN | Agent B | done | ARC | installed payload and embedded operations are disjoint |
| REL | Agent A/C | done | LEG, RUN | version and machine-image duplicate authorities removed |
| DOC | Agent C | done | final taxonomy | five-file documentation inventory passes |
| DEL | Agent C | done | LEG, BRD, RUN | MSI extraction and deterministic Setup build pass |
| INT | Coordinator | done | all rows | integrated gate and artifact hashes recorded |

## Blockers

| ID | Dependency | Owner | Status |
|---|---|---|---|
| B01 | Choose AGPL expression: this cycle uses `AGPL-3.0-only` from the user's “AGPL 3.0” instruction. | Coordinator | resolved |
| B02 | Hosted Windows CI is excluded by user instruction. | Coordinator | resolved |

## Handoffs

| From | To | Requirement | Status |
|---|---|---|---|
| A | C | Product license and copyright must be packaged by MSI/Burn. | done |
| B | C | Installer must stage the final runtime taxonomy and lock exactly. | done |
| A/B/C | Coordinator | One source/build gate and artifact inventory. | done |

## Residual acceptance

No source or build blocker remains. Hosted Windows CI is intentionally excluded.
Fresh install, upgrade, repair, reboot/resume and live tray behavior remain physical
acceptance on a disposable Windows host and are not inferred from build evidence.
