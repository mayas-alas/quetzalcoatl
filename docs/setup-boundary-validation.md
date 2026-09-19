# Setup boundary implementation evidence

Implemented on 2026-09-18 in the setup-ui-packaging worktree.

## Changes

- `src/bin/gnx-setup.rs`: opt-in `--json-progress` NDJSON transport, flushed
  started/completed events, fixed sanitized projection, unchanged preflight and
  exit semantics, explicit `apply_available: false`, and nonzero output failures.
- `tests/setup_cli.rs`: default report compatibility, rejection of apply and
  argument canaries, stream failure/exit preservation, and parity with preflight.
  A binary unit test checks all three report states and excludes report canaries.
- `packaging/windows/install.ps1`: authenticate and install `gnx-setup.exe`.
  The build script already builds/copies/hashes that executable and needed no edit.
- `docs/setup-boundary.md`: headless contract and thin presentation responsibilities.

## Verification

- PASS: `cargo +stable-x86_64-pc-windows-gnu check --locked --bin gnx-setup --test setup_cli`.
- PASS: GNU rustfmt check for both changed Rust files.
- PASS: PowerShell AST syntax parsing of `packaging/windows/install.ps1`.
- PASS: `git diff --check` (only Windows line-ending notices).
- BLOCKED: native `cargo test --locked --bin gnx-setup --test setup_cli`:
  default MSVC toolchain resolves Git coreutils `link.exe`, not the MSVC linker.
- BLOCKED: GNU equivalent test command: `dlltool.exe` was missing from PATH;
  retry with its bundled directory found it, but import-library generation then
  failed with `CreateProcess` (the bundle has no assembler).
- No tests were executed successfully; type checking is not runtime test evidence.
  No installer was run, no installation was changed, and no release build was made.

## Integration limitations

The strict current Windows bundle parser rejects artifacts produced by the
six-artifact build manifest. This was reported to the coordinator, who assigned
schema reconciliation to the separate provision/bundle owner and explicitly
directed this worker not to modify manifest validation.

This delivery is a headless UI boundary, not a desktop UI or a mutating setup
implementation. Apply remains unavailable. Full Windows/WSL installation and
release promotion evidence is still required.
