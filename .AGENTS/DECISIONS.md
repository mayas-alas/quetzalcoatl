# Architecture decisions

## D-001 — Product completion is cluster READY

Convergence completes only after controller cluster verification or member join.

## D-002 — Persist only the cluster contract

Protected state contains node identity, role, controller identity, tailnet and join checkpoint. Version 0.1.12 does not change the state schema.

## D-003 — One exact runtime payload

`runtime/payload` is the only source payload. Its physical files, manifest entries and Rust `PAYLOAD_FILES` allowlist must be identical.

## D-004 — Resume verifies rather than recreates

A persisted controller verifies its existing cluster. A member resumes against the pinned controller. Machine recreation remains limited to an incompatible managed generation.

## D-005 — Four crates remain the MVP boundary

0.1.12 improves internal modules but adds no crate. A module may become a crate only after its boundary survives build, upgrade, reboot and recovery evidence.

## D-006 — Fedora execution is on-demand and typed

The Windows service invokes `gnx-runtime-agent` over the existing Podman Machine SSH transport. Rust selects operations from `RuntimeOperation`; the agent has no listener and no generic exec operation.

## D-007 — Static shell programs use stdin, never dynamic `sh -c`

Bootstrap and probes may send fixed programs to `sh -s`. External values and secrets must use argv, stdin or fixed ephemeral files and must never be interpolated into a remote shell command string.

## D-008 — Remote process resources are bounded

Remote stdin and captured output have fixed limits. An operation exceeding the transport timeout is terminated and reported as a bounded error.

## D-009 — Build entry point remains stable

`installer/build.ps1` remains the single operator entry point. Dependency, contract, runtime, Rust, MSI and Burn implementation lives in dot-sourced modules that use the root supplied by the entry point.
