# Evidence — 0.1.15

## Source evidence included in the tree

- `ci/validate_repository.py`
- `ci/validate_runtime.py`
- `ci/validate_remote_execution.py`
- `ci/validate_cli_contract.py`
- `ci/validate_release_contract.py`
- `ci/validate_installer_resume.py`
- `ci/validate_cluster_contract.py`

## Runtime shell evidence

```sh
sh -n runtime/payload/bin/gnx-runtime-agent
sh -n runtime/payload/bin/gnx-pve-cluster-create
```

## Required Windows evidence

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
.\installer\build.ps1 -TestRebootContractOnly
.\installer\build.ps1
```

The physical acceptance run must retain:

- `C:\ProgramData\Quetzalcoatl\Installer\install-state.json`;
- stable WSL/Podman MSI logs;
- Burn and product MSI logs;
- `gnx --version`, `gnx status --json`;
- `pvecm status`, `pvecm nodes` and cluster resource evidence from the member test.
