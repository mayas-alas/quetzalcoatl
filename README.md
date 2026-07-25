# Quetzalcoatl 0.1.15

Quetzalcoatl is a Windows-managed MVP that provisions and reconciles a Fedora Podman Machine containing the Tailscale and Proxmox runtime used by controller/member nodes.

Version 0.1.15 is a recovery-first release. It stages pinned WSL and Podman installers in a GNX-owned stable cache before invoking Windows Installer, records bounded reboot/rerun state, adds local CLI version flags and closes the existing member join with explicit verification and confirmation.

The workspace remains at four Cargo packages. Protocol schema 2, persisted-state schema 2 and runtime generation `proxmox-cluster-v2` are unchanged.

## CLI

```powershell
gnx -v
gnx --version
gnx status
gnx status --json
gnx configure
gnx restart
```

## Build and validation

```powershell
python .\ci\validate_repository.py
python .\ci\validate_runtime.py
python .\ci\validate_remote_execution.py
python .\ci\validate_cli_contract.py
python .\ci\validate_release_contract.py
python .\ci\validate_installer_resume.py
python .\ci\validate_cluster_contract.py

cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
.\installer\build.ps1
```

See `docs/INSTALLER_RECOVERY.md`, `docs/MEMBER_MEMBERSHIP.md`, `docs/AUDIT_0.1.15.md` and `docs/VALIDATION.md`.
