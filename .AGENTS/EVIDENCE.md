# Evidence — 0.1.17

## Source evidence

- `crates/gnx-host-preflight/src/host_profile.rs` owns inventory, policy and persistence.
- `crates/gnx-service/src/runtime/profile.rs` owns service-side loading and validation.
- `runtime/host.rs` generates `.wslconfig`; `runtime/machine.rs` applies the same profile to Podman.
- `runtime/tailscale.rs` parses online controllers, generates the fixed Serve JSON and sends it through stdin to `tailscale serve set-raw`.
- `runtime/topology.rs` resolves controller/member state without a member-count limit.
- `runtime/reconciler.rs` starts PVE before applying and validating Serve.
- `runtime/remote/operation.rs` contains the closed Fedora-agent operation map.
- `runtime/remote/transport.rs` enforces bounded stdin/output, timeout, cancellation and zeroization.
- `ci/validate_remote_execution.py` rejects shell-string execution and shell-control syntax in direct remote argv arrays.
- `docs/REMOTE_EXECUTION.md` and `docs/REMOTE_EXECUTION_REVIEW.md` define the normative policy and review evidence.

## Required Windows evidence

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `cargo test --workspace --all-targets --locked`.
- `installer/build.ps1 -TestRebootContractOnly`.
- `installer/build.ps1`.
- Clean Dockur install showing the selected host profile, controller readiness and functional Tailscale HTTPS to PVE.
- A later Dockur node selecting member when an online controller exists.
