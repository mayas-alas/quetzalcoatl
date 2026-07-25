# Validation — 0.1.15

## Source checks

```powershell
python .\ci\validate_repository.py
python .\ci\validate_runtime.py
python .\ci\validate_remote_execution.py
python .\ci\validate_cli_contract.py
python .\ci\validate_release_contract.py
python .\ci\validate_installer_resume.py
python .\ci\validate_cluster_contract.py
```

## Rust checks

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

## Installer checks

```powershell
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
Get-ChildItem -Recurse -File | Unblock-File
.\installer\build.ps1 -TestRebootContractOnly
.\installer\build.ps1
```

The build verifies pinned dependencies, stable-staging wiring, reboot exit mappings, runtime payload hashes, release identities, administrative MSI extraction, service/CLI payload coherence and deterministic Burn metadata.

## Clean-host acceptance

1. Start setup on Windows 11 without WSL features.
2. Allow the requested reboot and confirm Burn resumes.
3. Confirm WSL and Podman MSIs are staged under the GNX `ProgramData` cache.
4. Confirm the dependency logs remain available after success or failure.
5. Confirm `gnx --version` works before contacting the service.
6. Configure the first host and reach controller `READY`.
7. Install/configure a second host and observe all member stages through `READY`.
8. Capture `pvecm status`, `pvecm nodes` and PVE cluster resource evidence.
9. Add another member or simulate its inventory to verify no count-based rejection.
10. Reboot the member and confirm reconciliation returns to `READY`.

## Required regression checks

- upgrade from installed 0.1.14 without losing protected configuration or managed runtime state;
- invalid/missing ancillary dependency payload;
- incompatible existing Podman registration;
- repeated failed phase reaches the bounded-attempt error;
- controller unavailable, duplicate identity and failed membership confirmation;
- uninstall and secret/cache residue inspection.
