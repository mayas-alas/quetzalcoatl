# Reconciler recovery

## Source requirement

The 0.1.11 sequence is moved intact into `runtime/reconciler.rs`. `runtime/mod.rs` starts the stage and maps final errors only.

## Acceptance scenarios

- compatible machine resumes without recreation;
- incompatible machine preserves Tailscale state, resets the cluster checkpoint and reapplies payload;
- persisted controller verifies the existing cluster;
- joining member resumes against its pinned controller;
- repeated payload application is hash-verified and atomic;
- reboot during convergence returns to a valid stage and reaches READY.

The source release implements the boundary. Physical scenario evidence remains mandatory on Windows/Fedora hosts.
