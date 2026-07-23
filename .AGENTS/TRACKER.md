# PoC/MVP closure tracker

Updated: 2026-07-22

## Score

Current verified score: **50/100**

The score measures verified closure evidence, not code volume or agent-reported completion.

| Gate | Weight | Status | Verified points | Required evidence |
|---|---:|---|---:|---|
| G1 Scope, agent control, and normalized docs | 5 | VERIFIED | 5 | Consistent committed sources with legacy routing |
| G2 GitHub Actions Dockur compatibility and noVNC | 10 | PARTIAL | 0 | Three successful hosted install/resume runs exist, but WSL2/Podman could not start under the runner's nested virtualization |
| G3 Persisted member, deterministic discovery, and status | 25 | VERIFIED | 25 | Unit/integration tests and status fixtures |
| G4 Secure Proxmox join and controller-only writer | 20 | VERIFIED | 20 | Idempotence, restart, denial, and secret-leak tests |
| G5 Three real hosts, quorum, reboots, and security | 30 | BLOCKED | 0 | Three-host evidence bundle from the physical lab |
| G6 Pilotability: signing, preflight, diagnostics, lifecycle | 10 | IN_PROGRESS | 0 | Signed hashes, diagnostics, guide, lifecycle runs |

Functional MVP is reached at 90/100 only when G1-G5 are all `VERIFIED`. Pilotable MVP is 100/100. The score cannot exceed 70/100 until G5 is verified.

## Active work

| Work item | Owner | State | Dependency | Output |
|---|---|---|---|---|
| GOV-01 Establish agent contract and tracker | architect | VERIFIED | none | `AGENTS.md`, `.AGENTS/` |
| MVP-CLOSE 0.1.4 candidate and focused documentation | architect | LOCAL_VERIFIED | installed 0.1.3 controller failure | byte-reproducible EXE/MSI and normalized sources; live upgrade pending |
| OPERATOR-CONTROL 0.1.5 native recovery controls | architect/operator | LOCAL_VERIFIED | live controller upgrade | `gnx restart`, resumable `gnx configure forgejo`, byte-reproducible installer |
| CI-01 Dockur Actions lane | architect/operator | PARTIAL | reproducible 0.1.4 candidate | prior 0.1.3 runs retained; 0.1.4 rerun pending |
| CORE-01 Persisted member and status contract | codex-cli-state | VERIFIED | GOV-01 | Rust changes and tests |
| CORE-02 Deterministic member discovery/orchestration | codex-cli-runtime | VERIFIED | CORE-01 | Rust changes and tests |
| PVE-01 Secure resumable Proxmox join | codex-cli-pve/architect | VERIFIED | CORE-01 | Payload changes and tests |
| INT-01 Integrate controller/member paths | codex-cli-integration/architect | VERIFIED | CORE-02, PVE-01 | Workspace test evidence |
| PKG-01 Prior integrated installer baseline | architect | VERIFIED | INT-01 | historical MSI/Setup hashes |
| PKG-03 Reproducible 0.1.3 installer identity | architect | VERIFIED | INT-01 | historical fixed identities and byte-identical MSI/Setup rebuild |
| PKG-04 Reproducible 0.1.4 installer and upgrade identity | architect | LOCAL_VERIFIED | RT-01 | new identities and byte-identical MSI/Setup; live 0.1.3 upgrade pending |
| RT-01 Serialize controller OpenTofu apply and retain useful failure stage | architect/operator | LOCAL_VERIFIED | installed 0.1.3 controller failure | corrected 0.1.4 payload; live controller rerun pending |
| LAB-01 Three-host network and cluster acceptance | architect/operator | BLOCKED | CI-01, INT-01, three hosts | Physical evidence bundle |
| DOC-01 Consolidate documentation authorities | architect | VERIFIED | MVP-CLOSE | ten maintained Markdown sources |

## Current blockers

- The exact 0.1.3 hash completed successful interactive Dockur runs for `controller`, `member-1`, and `member-2`, including the real two-reboot installer path. All three guests entered Windows automatic repair when the nested Windows hypervisor started; disabling `hypervisorlaunchtype` recovered Windows and allowed installation/evidence export, but left `podman_machine=failed` before role resolution. This hosted limitation keeps G2 partial and does not prove controller/member runtime convergence.
- Three physical consumer Windows 11 hosts have not yet supplied the required network and quorum evidence for this cycle.
- The current integrated bundle is unsigned and has not completed current-candidate repair/uninstall/recovery runs, so G6 receives no points yet.
- A physical Windows controller reached quorate PVE but failed during the
  controller OpenTofu one-shot. The retained status bounded the reverse-order
  journal before the provider diagnostic, so the exact `tofu` error was lost.
  RT-01 serializes provider mutations with `-parallelism=1`, retains a
  stage-specific chronological diagnostic and passed the local reproducible
  package gate. The correction remains live-unverified until the rebuilt 0.1.4
  controller reaches `READY`.
- Existing GitHub runner evidence used DERP with approximately 64-73 ms RTT and cannot close G5.
