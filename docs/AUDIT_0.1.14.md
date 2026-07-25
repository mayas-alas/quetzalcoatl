# Audit report — 0.1.14

## Closed

- CLI monolith split by command, output and transport.
- Protocol models split without changing schema 2.
- Host-preflight path aligned with its package identity.
- Service composition separated from IPC, secrets, state and runtime.
- Runtime startup exposed through a narrow control facade.
- Legacy replaced paths prohibited by validation.
- Cargo and installer paths updated coherently.

## Deliberately unchanged

Runtime payload, remote-operation allowlist, reconciliation stages, state schema, runtime generation, controller/member semantics and CLI command set.

## External certification still required

Windows fmt/Clippy/tests, full WiX build and an upgrade from installed 0.1.13 with runtime state preserved.
