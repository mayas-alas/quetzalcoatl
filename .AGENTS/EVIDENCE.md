# Evidence — 0.1.14

## Static evidence included in the source tree

- `ci/validate_repository.py`
- `ci/validate_runtime.py`
- `ci/validate_remote_execution.py`
- `ci/validate_cli_contract.py`
- `ci/validate_release_contract.py`

## Required Windows evidence

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
.\installer\build.ps1 -TestRebootContractOnly
.\installer\build.ps1
```

Install 0.1.14 over the working 0.1.13 installation and confirm `gnx status`, `gnx status --json`, `gnx restart`, configuration preservation and runtime readiness.
