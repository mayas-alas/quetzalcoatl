# 0.2.13 security scope

## Outcome

Harden the privileged Windows maintenance and local-control surfaces without
changing the product topology. Deliver one coherent source tree and one
`QuetzalcoatlSetup.exe` that owns fresh install, upgrade, repair and uninstall.

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
| STG | Protected privileged staging | Installer state/cache use protected ACLs, reject every reparse-point component and execute only a revalidated locked artifact. |
| IPC | Bounded local control | Named Pipe remains local/authenticated and a stalled client cannot block subsequent clients indefinitely. |
| SHD | Cooperative shutdown | Stop rejects event precreation, stops accepting IPC and joins reconciliation without `process::exit`. |
| SUP | Verifiable supply chain | Dependency advisories and Authenticode policy are explicit release gates; unsigned production output fails closed. |
| REL | Integrated release | Version, identities, build, tests, hashes and physical evidence agree on 0.2.13. |

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
| Writable privileged installer staging | 5 | 5 | 5 | 5 | 35 | P0 | STG |
| Unsigned release artifacts | 5 | 4 | 5 | 5 | 30 | P1 | SUP |
| Single blocking Named Pipe client | 4 | 4 | 4 | 5 | 25 | P1 | IPC |
| Abrupt service termination | 5 | 3 | 4 | 5 | 24 | P1 | SHD |
| Dependency advisory status unknown | 4 | 3 | 4 | 5 | 21 | P1 | SUP |

## Definition of done

All included rows are `done`; `tools/check.ps1` passes; production artifacts have
valid Authenticode signatures; final hashes are recorded; hostile local staging
tests pass; uninstall removes every product-owned surface; reinstall and repair
recover the same READY controller and quorate cluster.
