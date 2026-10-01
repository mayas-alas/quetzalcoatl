# GnX Node v0.1.18

## Managed runtime controls

- Adds `gnx runtime stop --confirm` to stop only the managed GnX WSL distribution and release its RAM.
- Adds `gnx shutdown --confirm` as an equivalent explicit alias.
- Adds `gnx runtime start` to start and health-check the managed runtime.
- The host agent stops only allowlisted GnX units and terminates only `gnx-node` under `gnxnodesvc`; it never runs global `wsl --shutdown`.
- A stopped runtime is persisted as `RuntimeStopped`; host keepalive and status verification do not restart it implicitly.

## Validation

- `cargo fmt --all -- --check`
- `cargo test --workspace --locked`
- `node tests/test-static-contracts.mjs`
- `installer/scripts/build.ps1`
- `installer/scripts/verify.ps1`
