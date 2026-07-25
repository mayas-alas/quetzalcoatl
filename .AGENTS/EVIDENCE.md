# Evidence — 0.1.13

## Static source evidence

```powershell
python .\ci\validate_repository.py
python .\ci\validate_runtime.py
python .\ci\validate_remote_execution.py
python .\ci\validate_cli_contract.py
python .\ci\validate_release_contract.py
```

## Windows build evidence

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
.\installer\build.ps1 -TestRebootContractOnly
.\installer\build.ps1
```

The MSI administrative extraction must prove that both `gnx-service.exe` and `gnx.exe` match the freshly built SHA-256 values.

## Upgrade evidence

Install 0.1.13 over the working 0.1.12 installation. Confirm protected configuration, managed machine and role remain intact; run `gnx status` and `gnx status --json`; then verify restart and runtime readiness.
