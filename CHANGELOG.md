# Changelog

## 0.1.12 — MVP consolidation

- Extracted runtime orchestration into `reconciler.rs` without changing stage order.
- Separated runtime errors from data models.
- Replaced free-form Fedora-agent argv construction with a closed `RuntimeOperation` enum.
- Added remote stdin/output limits and a 15-minute process timeout.
- Removed managed-runtime `sh -c` execution, including PVE credential configuration.
- Bumped the exact payload manifest contract to version 4 and regenerated hashes.
- Split Rust build and payload-source verification into installer modules.
- Added local repository, runtime, remote-execution and release-contract validators.
- Updated 0.1.12 MSI/Burn identities while retaining upgrade families.

## 0.1.11 — scoped modularization

- Split the runtime monolith inside `gnx-service`.
- Consolidated one runtime payload and added the on-demand Fedora agent.
- Split installer build helpers without changing its entry point.
