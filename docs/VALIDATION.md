# Validation

## CI checks

```bash
python3 ci/validate_runtime.py
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
find runtime/payload-v1/bin -type f -print0 | xargs -0 -n1 sh -n
shellcheck runtime/payload-v1/bin/*
```

The runtime validator checks:

- manifest JSON and version;
- the exact three-component set;
- equality between the 11 manifest entries, the physical payload and the Rust allowlist;
- SHA-256 values and Unix modes;
- the persisted controller `READY` checkpoint;
- controller cluster verification on resume.

## Manual Windows acceptance

On a clean compatible Windows 11 host:

1. Install the bundle.
2. Run elevated `gnx configure` and provide the tailnet, auth key and PVE password.
3. Observe `gnx status` until `READY`.
4. Verify PVE through the approved tailnet HTTPS route.
5. Restart Windows and confirm the same role and controller identity return to `READY`.

For member nodes, verify a direct Tailscale path, synchronized clocks, usable MTU, TCP 22/8006, Corosync UDP 5405-5412 and quorum. Hosted CI cannot prove these physical network properties.
