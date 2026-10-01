# GnX Node v0.1.19

## Runtime memory lifecycle

- Prevents the host keepalive and status monitor from reopening WSL while `gnx runtime stop --confirm` is terminating the managed runtime.
- Adds WSL page reporting and gradual automatic memory reclaim to the managed `gnxnodesvc` profile configuration.
- Retains the strict boundary: only the managed `gnx-node` runtime is stopped; global `wsl --shutdown` is never used.

## Host validation

On the installed host, `runtime stop --confirm` changed the durable status to `RuntimeStopped` and reduced `vmmemWSL` working set from 2399.7 MiB to 1205.0 MiB after 15 seconds. `runtime start` returned `NodeReady`; `https://app.gnx` returned HTTP 200.

## Build validation

- `cargo fmt --all -- --check`
- `cargo test --workspace --locked`
- `node tests/test-static-contracts.mjs`
- `installer/scripts/build.ps1`
- `installer/scripts/verify.ps1`
