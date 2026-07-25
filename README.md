# Quetzalcoatl 0.1.14

Quetzalcoatl is a Windows-managed MVP that provisions and reconciles a Fedora Podman Machine containing the Tailscale and Proxmox runtime used by controller/member nodes.

Version 0.1.14 is a behavior-preserving structural release. It keeps the four-package workspace and all installed contracts, while separating CLI commands, protocol models and Windows service responsibilities into explicit modules.

## Build and validation

```powershell
python .\ci\validate_repository.py
python .\ci\validate_runtime.py
python .\ci\validate_remote_execution.py
python .\ci\validate_cli_contract.py
python .\ci\validate_release_contract.py

cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
.\installer\build.ps1
```

See `docs/ARCHITECTURE.md`, `docs/AUDIT_0.1.14.md` and `docs/TARGET_0.2.md`.
