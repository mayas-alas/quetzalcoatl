# 0.2.0 release-hardening scope

## Outcome

Deliver one coherent 0.2.0 source tree and one validated
`QuetzalcoatlSetup.exe`. Setup remains the sole user-facing installation, upgrade
and repair interface.

## Included

| ID | Remediation | Acceptance |
|---|---|---|
| LEG | Product licensing and notices | Root AGPLv3 license, Hector AB copyright, Cargo/PE/MSI metadata and separate third-party notices. |
| BRD | Canonical branding assets | One `installer/assets` tree contains canonical branding sources and installer-specific derivatives. |
| ARC | Real module boundaries | No `#[path]` cross-layer wiring or broad production glob imports; domain and infrastructure are crate modules. |
| TYP | Contract typing | Lifecycle, health and PVE URL validation use closed types at boundaries; invalid values fail closed. |
| RUN | Runtime taxonomy and locks | Installed files and orchestration operations are distinct; one authoritative machine-image fact set; LF policy covers all runtime sources. |
| REL | Release-source consolidation | Version/copyright metadata is derived where possible; redundant `VERSION` and static fixtures are removed or consumed directly. |
| DOC | Documentation reduction | Minimal authoritative architecture, contracts, operations and validation documents. |
| DEL | Delivery assurance | Source gates, MSI extraction, Burn identity, branding, install/upgrade/repair/restart contracts and final hashes pass. |

## Preserved invariants

- Exactly four Cargo packages.
- Protocol schema 2, Named Pipe command set and persisted-state schema 2.
- Host-profile schema 1, runtime generation `proxmox-cluster-v2` and payload contract 5.
- Closed remote argv, bounded stdin/output/time and atomic durable state.
- New-node role from online controller presence only.
- PVE readiness before Tailscale Serve.
- Bounded installer resume and member-join recovery.
- No localhost UI, listener, new product port or Tauri runtime.
- `installer/build.ps1` remains the release entry point.

## Explicit exclusion

Hosted CI on another Windows host is not part of this delivery. The local
`tools/check.ps1` gate remains mandatory and suitable for future CI adoption.

## Risk matrix

`score = impact × probability + urgency + necessity`, each factor 1–5.

| Risk | I | P | U | N | Score | Priority | Gate |
|---|---:|---:|---:|---:|---:|---|---|
| Incorrect product/third-party licensing | 5 | 5 | 5 | 5 | 35 | P0 | LEG |
| CRLF corrupts Linux runtime programs | 5 | 4 | 5 | 5 | 30 | P0 | RUN |
| Cosmetic folders hide coupled Rust modules | 5 | 4 | 4 | 5 | 29 | P0 | ARC |
| Upgrade/repair replaces an incomplete product | 5 | 4 | 4 | 5 | 29 | P0 | DEL |
| Invalid string state crosses a contract boundary | 4 | 4 | 3 | 5 | 24 | P1 | TYP |
| Branding/version facts drift between tools | 4 | 4 | 3 | 5 | 24 | P1 | BRD/REL |
| Runtime facts are duplicated in lock and source | 4 | 3 | 3 | 5 | 20 | P1 | RUN |
| Documentation and fixtures duplicate behavior | 3 | 4 | 2 | 4 | 18 | P2 | DOC/REL |

## Definition of done

All included rows are `done`; `tools/check.ps1` passes; final MSI and Setup hashes
are recorded; physical fresh-install, upgrade, repair, reboot and tray checks are
reported as executed or explicitly pending, never inferred from source tests.
