# Structural refactor

## Required moves

- `crates/host-preflight` → `crates/gnx-host-preflight`
- `gnx-cli/src/pipe.rs` → `gnx-cli/src/client.rs`
- split CLI command and output responsibilities
- split protocol request, response, status and version models
- `gnx-service/src/pipe.rs` → `gnx-service/src/ipc/mod.rs`
- `secrets.rs` and `state.rs` → module directories
- add service composition and runtime-control boundaries

No capability expansion is permitted.
