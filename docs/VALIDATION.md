# Validation — 0.1.17

## Source checks

```powershell
python .\ci\validate_repository.py
python .\ci\validate_runtime.py
python .\ci\validate_remote_execution.py
python .\ci\validate_cli_contract.py
python .\ci\validate_release_contract.py
python .\ci\validate_installer_resume.py
python .\ci\validate_cluster_contract.py
python .\ci\validate_host_profile.py
```

`validate_remote_execution.py` must reject:

- free-form Fedora-agent arguments;
- `sh` or `bash` followed by `-c`, including multiline Rust arrays;
- shell-control operators embedded in direct remote argv arrays;
- missing input/output/timeout/cancellation guards;
- runtime-agent listener or arbitrary execution behavior.

## Rust checks

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

Remote-operation tests should assert exact argv, stdin behavior, representative payload parsing, failure mapping and absence of sensitive data in errors.

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
6. Configure the first host and confirm it observes zero online controllers.
7. Confirm the final Tailscale name is observed before controller state is committed.
8. Confirm PVE is ready on local port 8006 before Serve is applied.
9. Confirm `tailscale serve status --json` contains only the fixed HTTPS PVE proxy and Funnel is disabled.
10. Install/configure a second host and confirm it becomes a member when the controller is online.
11. Add another member and verify member count never blocks discovery.
12. Capture `pvecm status`, `pvecm nodes` and PVE cluster resource evidence.
13. Reboot the member and confirm reconciliation returns to readiness without controller mutation.

## Remote-execution acceptance

For every new or changed remote operation, attach the completed template from `REMOTE_EXECUTION_REVIEW.md` and verify:

- the executable layer is correct;
- argv is fixed and contains no shell syntax;
- variable data reaches the intended process through stdin;
- any path exists in the documented layer and is truly required;
- timeout, limits, redaction and idempotency behave as documented.

## Required regression checks

- upgrade from installed 0.1.14/0.1.15 without losing protected configuration or managed runtime state;
- invalid/missing ancillary dependency payload;
- incompatible existing Podman registration;
- repeated failed phase reaches the bounded-attempt error;
- controller unavailable, duplicate local identity and failed membership confirmation;
- PVE unavailable prevents Serve publication;
- malformed Serve JSON or non-zero `set-raw` exit is reported without dumping input;
- uninstall and secret/cache residue inspection.
