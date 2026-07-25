# Validation — 0.1.13

## Source checks

```powershell
python .\ci\validate_repository.py
python .\ci\validate_runtime.py
python .\ci\validate_remote_execution.py
python .\ci\validate_cli_contract.py
python .\ci\validate_release_contract.py
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

The build clears Mark-of-the-Web from the pinned `.config/dotnet-tools.json` manifest before restoring WiX. It also verifies source payload hashes before compiling Rust, static CRT imports, MSI identities, administrative extraction, installed service, CLI and payload coherence and deterministic Burn metadata.

## Required acceptance outside source validation

- clean Windows 11 install and CLI/service Named Pipe operation;
- upgrade from installed 0.1.12 without losing protected state or the managed machine;
- runtime-agent handshake and operation rejection behavior in Fedora;
- controller cluster creation and persisted verification after reboot;
- member join and resume against the pinned controller;
- uninstall and secret-residue inspection.
