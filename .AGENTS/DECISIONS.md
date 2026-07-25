# Decisions — 0.1.14

1. `crates/` remains the correct Rust workspace convention; executables and libraries are not split into `apps/` until 0.2.x.
2. The workspace remains at four packages.
3. `crates/host-preflight` becomes `crates/gnx-host-preflight`; the package and executable names remain unchanged.
4. CLI commands are modules, not new binaries or protocol operations.
5. `gnx-service` remains one crate but receives explicit composition zones.
6. IPC cannot call Podman, Tailscale or Proxmox implementation modules directly.
7. Runtime reconciliation order, public stages and error codes are unchanged.
8. MSI/Burn upgrade families are retained while 0.1.14 receives new release identities.
