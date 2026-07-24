# Release 0.1.12 — MVP consolidation

## Goal

Turn the 0.1.11 modularization into enforceable runtime and release boundaries while preserving the working product behavior.

## Required changes

1. Extract the runtime sequence into `reconciler.rs`.
2. Separate `GateError` from runtime data models.
3. Replace free-form runtime-agent argument arrays with `RuntimeOperation`.
4. Rename the process boundary to `remote/transport.rs` and add limits/timeout.
5. Remove PVE credential shell-string execution.
6. Bump the exact runtime payload contract to version 4 and regenerate hashes.
7. Extract Rust build and source-payload verification into installer modules.
8. Generate new 0.1.12 release identities while retaining upgrade families.
9. Update agent records, architecture and local validators.

## Non-goals

No new crate, state schema, machine generation, CLI command, daemon, listener, OpenTofu, tray UI or hosted pipeline.
