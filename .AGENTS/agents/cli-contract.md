# Agent: CLI contract

## Ownership

- `crates/gnx-cli/` and the CLI-facing protocol contract;
- `installer/package.wxs` CLI component and PATH registration;
- MSI extraction verification for `gnx.exe`;
- `ci/validate_cli_contract.py`.

## Invariants

- Supported commands remain `status`, `configure` and `restart`.
- `status --json` and human status expose the same runtime state.
- Configuration secrets are never echoed.
- CLI responses must match protocol schema v2.
- The installed `gnx.exe` must match the freshly built binary.
