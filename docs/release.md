# Release 0.3.1

0.3.1 is a scope-cleanup release over 0.3.0. It does not add a new capability.

## Changes

- Public architecture is only **Access / Control / Compute**.
- Stable Compute entrypoint becomes `https://compute.gnx`.
- Node intent no longer selects release images.
- Release-specific immutable image references move to one internal release file.
- `gnx.toml` defaults runtime paths and internal service addresses instead of requiring them from the operator.
- The optional private identity assertion is documented as `network.identity_ip`.
- Windows defaults to the dedicated Linux distribution `GNX` rather than a general-purpose distribution name.
- Documentation is reduced to architecture, PoC acceptance and Windows build instructions.

## Unchanged scope

0.3.1 keeps the existing single runtime and the current Compute implementation. It does not add providers, plugins, workload provisioning, HA, auto-update or a commercial installer.

## Verification status

This bundle was prepared by static source editing only. No Rust compiler, Cargo command, runtime, container engine or PoC gate was executed while preparing it. Build and acceptance remain operator evidence.
