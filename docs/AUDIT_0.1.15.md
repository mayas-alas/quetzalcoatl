# Audit report — 0.1.15

## Closed in source

- WSL and Podman no longer execute directly as Burn MSI packages.
- Dependency payloads are fixed-name, exact-size and SHA-256 validated before and after stable staging.
- Staging uses a GNX-owned `ProgramData` cache and persistent verbose MSI logs.
- A product-version-scoped journal bounds repeated phases to three attempts.
- Compatible dependency registrations are reused and post-validated; incompatible registrations fail closed.
- `gnx -v` and `gnx --version` are local CLI actions.
- Member join exposes persisted prepare, authorize, join, verify and confirm stages.
- Membership confirmation is a typed runtime-agent operation, not arbitrary remote execution.
- Multiple members are allowed while zero/multiple controllers remain unsupported.
- Protocol schema 2, persisted-state schema 2 and runtime generation remain unchanged.

## Deliberately unchanged

The Named Pipe command set, public status/configure/restart behavior, PVE root credential mechanism, `pvecm add` join path, four-package workspace and MSI/Burn upgrade families.

## Residual risk

- Burn ancillary-payload placement and helper execution require a real Windows build/run.
- The stable cache mitigates the observed path failure but cannot guarantee the host filesystem or Windows Installer itself is healthy.
- Nested virtualization is not certified by MSI installation; Podman Machine and `/dev/kvm` remain runtime gates.
- Two-node quorum is suitable for the first join test but is not an HA claim.
- Controller/member confirmation is based on distributed PVE cluster state and does not add a separate controller authorization service.

## External certification required

Rust fmt/Clippy/tests, full WiX build, clean install with reboot/resume, upgrade from 0.1.14, Podman Machine startup, first member join, an additional member discovery test and uninstall/residue inspection.
