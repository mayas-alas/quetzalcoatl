# PoC/MVP closure tracker

Updated: 2026-07-22

## Score

Current verified score: **50/100**

The score measures verified closure evidence, not code volume or agent-reported completion.

| Gate | Weight | Status | Verified points | Required evidence |
|---|---:|---|---:|---|
| G1 Scope, agent control, and normalized docs | 5 | VERIFIED | 5 | Consistent committed sources with legacy routing |
| G2 GitHub Actions Dockur compatibility and noVNC | 10 | IN_PROGRESS | 0 | Successful run URL, pinned image, artifacts, end-user smoke |
| G3 Persisted member, deterministic discovery, and status | 25 | VERIFIED | 25 | Unit/integration tests and status fixtures |
| G4 Secure Proxmox join and controller-only writer | 20 | VERIFIED | 20 | Idempotence, restart, denial, and secret-leak tests |
| G5 Three real hosts, quorum, reboots, and security | 30 | BLOCKED | 0 | Three-host evidence bundle from the physical lab |
| G6 Pilotability: signing, preflight, diagnostics, lifecycle | 10 | IN_PROGRESS | 0 | Signed hashes, diagnostics, guide, lifecycle runs |

Functional MVP is reached at 90/100 only when G1-G5 are all `VERIFIED`. Pilotable MVP is 100/100. The score cannot exceed 70/100 until G5 is verified.

## Active work

| Work item | Owner | State | Dependency | Output |
|---|---|---|---|---|
| GOV-01 Establish agent contract and tracker | architect | VERIFIED | none | `AGENTS.md`, `.AGENTS/` |
| MVP-CLOSE 0.1.3 candidate and focused documentation | architect | LOCAL_VERIFIED | reviewed member implementation | frozen EXE/MSI and normalized sources |
| CI-01 Dockur Actions lane | architect/operator | IN_PROGRESS | frozen 0.1.3 candidate | GitHub workflow and run evidence |
| CORE-01 Persisted member and status contract | codex-cli-state | VERIFIED | GOV-01 | Rust changes and tests |
| CORE-02 Deterministic member discovery/orchestration | codex-cli-runtime | VERIFIED | CORE-01 | Rust changes and tests |
| PVE-01 Secure resumable Proxmox join | codex-cli-pve/architect | VERIFIED | CORE-01 | Payload changes and tests |
| INT-01 Integrate controller/member paths | codex-cli-integration/architect | VERIFIED | CORE-02, PVE-01 | Workspace test evidence |
| PKG-01 Prior integrated installer baseline | architect | VERIFIED | INT-01 | historical MSI/Setup hashes |
| PKG-03 Frozen 0.1.3 installer identity | architect | VERIFIED | MVP-CLOSE | fixed ProductCode, MSI/Setup hashes |
| LAB-01 Three-host network and cluster acceptance | architect/operator | BLOCKED | CI-01, INT-01, three hosts | Physical evidence bundle |
| DOC-01 Consolidate documentation authorities | architect | VERIFIED | MVP-CLOSE | ten maintained Markdown sources |

## Current blockers

- WiX 5 changes PackageCode, bundle registration ID and timestamps on each bind; the frozen 0.1.3 hashes must be published and tested without rebuilding the candidate.
- The published Dockur workflow still needs a successful run and interactive noVNC smoke against that exact hash before G2 receives points.
- Three physical consumer Windows 11 hosts have not yet supplied the required network and quorum evidence for this cycle.
- The current integrated bundle is unsigned and has not completed current-candidate repair/uninstall/recovery runs, so G6 receives no points yet.
- Existing GitHub runner evidence used DERP with approximately 64-73 ms RTT and cannot close G5.
