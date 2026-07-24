# Release 0.1.11 — scoped modularization

## Goal

Deliver a buildable source release derived from 0.1.10 that removes structural duplication and monoliths without changing the MVP product contract.

## Required changes

1. Split `gnx-service/src/runtime_gate.rs` into responsibility-based modules.
2. Keep all runtime behavior inside the existing `gnx-service` crate.
3. Consolidate the active runtime into `runtime/payload` and delete both versioned source directories.
4. Add and verify the on-demand Fedora runtime agent.
5. Route existing fixed PVE and Tailscale script calls through the agent.
6. Split `installer/build.ps1` implementation functions into four modules.
7. Remove GitHub Actions from this release tree.
8. Update 0.1.11 versions, identities, docs and local validators.

## Non-goals

No OpenTofu, generalized OCI management, tray UI, GitHub Actions, new crates, new services, remote listener or persistence redesign.

## Exit criteria

- `python ci/validate_runtime.py`
- `python ci/validate_repository.py`
- Rust format, Clippy and workspace tests on the pinned toolchain
- `installer/build.ps1 -TestRebootContractOnly` on Windows
- full installer build and administrative extraction on Windows
