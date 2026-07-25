# CLI contract audit

## Closed condition

- `gnx status`, `gnx status --json`, `gnx configure` and `gnx restart` remain available.
- Human status includes all component, controller and cluster fields.
- CLI rejects a service response with a mismatched protocol schema.
- MSI contains one keyed `gnx.exe`, registers the installation folder in system PATH and verifies its extracted SHA-256.
- `ci/validate_cli_contract.py` passes.
