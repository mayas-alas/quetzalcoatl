# Agent execution contract — 0.1.14

This directory records the three bounded workstreams used for the 0.1.14 structural refactor. The release preserves the four-package workspace, protocol schema 2, persisted-state schema, runtime generation and existing CLI behavior.

## Workstreams

1. `cli-protocol-preflight.md` — CLI, protocol and host-preflight ownership.
2. `service-boundaries.md` — Windows service composition and internal module boundaries.
3. `release-integrity.md` — Cargo, WiX, validators, documentation and source packaging.

The integrator owns cross-workstream files, final validation and the release archive.

Closed 0.1.13 execution records are retained under `.AGENTS/archive/0.1.13/` and are not active instructions.
