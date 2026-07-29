# 0.2.12 maintenance scope

## Outcome

Deliver one coherent source tree and one `QuetzalcoatlSetup.exe` that owns fresh
install, upgrade, repair and uninstall. Setup remains the only user-facing
maintenance surface.

## Included

| ID | Outcome | Acceptance |
|---|---|---|
| LEG | Correct product licensing | AGPL-3.0-only, Hector AB notice and separate third-party notices. |
| BRD | Canonical branding | One `installer/assets` tree supplies MSI, Burn, tray and executable branding. |
| ARC | Clear module boundaries | Four Cargo packages; no parallel or version-suffixed implementations. |
| RUN | Closed runtime operations | Typed local/remote operations, bounded stdin/output/time and atomic durable files. |
| REC | Safe recovery | Runtime lock and machine image validate before service start; compatible state survives maintenance. |
| RBT | Bounded restart | Feature detection and resume do not loop or request redundant restarts. |
| ARP | Sole maintenance entry | One visible Setup registration; internal MSI remains hidden. |
| UNS | Complete uninstall | Product files, root, service, tray, PATH, startup, registrations and caches are removed. WSL, Podman, managed VM data and durable GNX state remain. |
| REL | Integrated release | Version, identities, build, tests, hashes and physical evidence agree on 0.2.12. |

## Preserved invariants

- Protocol schema 2, persisted-state schema 2 and host-profile schema 1.
- Runtime generation `proxmox-cluster-v2` and payload contract 5.
- Closed remote argv; variable data uses bounded stdin.
- Controller/member role derives only from validated online topology.
- PVE readiness precedes Tailscale Serve.
- No localhost UI, listener, new product port or Tauri runtime.
- `installer/build.ps1` remains the release entry point.

## Exclusion

Hosted CI on another Windows host is excluded. The local `tools/check.ps1` gate
remains mandatory and CI-ready.

## Priority matrix

`score = impact × probability + urgency + necessity`, each factor 1–5.

| Risk | I | P | U | N | Score | Priority | Gate |
|---|---:|---:|---:|---:|---:|---|---|
| Inherited WSL process locks product root | 5 | 5 | 5 | 5 | 35 | P0 | UNS |
| Service stop waits forever | 5 | 4 | 5 | 5 | 30 | P0 | UNS/RBT |
| Internal MSI exposes a second uninstall | 5 | 4 | 5 | 5 | 30 | P0 | ARP |
| Runtime payload starts incomplete | 5 | 4 | 4 | 5 | 29 | P0 | REC |
| Identity/version drift | 4 | 4 | 3 | 5 | 24 | P1 | REL |
| Documentation duplicates implementation | 3 | 3 | 2 | 4 | 15 | P2 | ARC |

## Definition of done

All included rows are `done`; `tools/check.ps1` passes; final hashes are recorded;
physical uninstall removes every product-owned surface; reinstall and repair
recover the same READY controller and quorate cluster.
