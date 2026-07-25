# Agent: service boundaries

## Ownership

- `crates/gnx-service/**`

## Mission

Turn the service crate into a clear composition root with `service`, `ipc`, `secrets`, `state` and `runtime` zones. Preserve all functions and keep the reconciler sequence literal. IPC must use service-level boundaries and cannot reach infrastructure implementation modules.

## Prohibited changes

- No state-schema migration.
- No runtime-generation change.
- No new remote operation.
- No controller/member semantic change.
- No payload change.
