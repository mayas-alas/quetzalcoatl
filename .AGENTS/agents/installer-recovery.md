# Agent: installer recovery

## Ownership

- `crates/gnx-host-preflight/`
- dependency helper sections of `installer/bundle.wxs`
- dependency staging contracts in `installer/modules/contracts.ps1`
- `ci/validate_installer_resume.py`
- `docs/INSTALLER_RECOVERY.md`

## Mission

Prevent WSL or Podman from being executed directly from an unstable Burn Package Cache path. Stage only the two pinned MSI payloads into the GNX-owned cache, validate size and SHA-256, execute `msiexec` with a persistent verbose log, and preserve bounded recovery state across reboot or rerun.

## Prohibited

- arbitrary source/destination paths;
- modifying global Package Cache ACLs;
- downloading dependencies at install time;
- accepting unpinned versions;
- adding a new bootstrapper application.

## Result

Integrated. The helper modes are closed (`install-wsl`, `install-podman`) and the source/build validators reject direct dependency MSI execution.
