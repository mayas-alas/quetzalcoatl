# Evidence — 0.1.12

## Local source evidence

Run:

```powershell
python .\ci\validate_repository.py
python .\ci\validate_runtime.py
python .\ci\validate_remote_execution.py
python .\ci\validate_release_contract.py
```

These checks prove the four-crate scope, exact payload file set and hashes, typed runtime operations, absence of `sh -c`, bounded transport markers, module boundaries and coherent release identities.

## Windows build evidence

Capture:

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
.\installer\build.ps1 -TestRebootContractOnly
.\installer\build.ps1
```

Retain MSI administrative extraction results and final MSI/Burn SHA-256 values.

## Runtime evidence

Validate clean install, service/CLI pipe, runtime-agent handshake, controller creation, member join, reboot resume, upgrade from 0.1.11 and uninstall. Do not mark these complete from static source checks alone.
