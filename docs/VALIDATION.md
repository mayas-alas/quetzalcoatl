# Validation

`tools/check.ps1` is the sole validation entry point.

Default execution performs:

1. repository taxonomy and four-package inventory;
2. release, IPC, persisted-state and CLI contracts;
3. remote-execution policy;
4. runtime lock, topology and lifecycle order;
5. installer maintenance, key paths, branding and tray contract;
6. `cargo fmt`, Clippy with warnings denied, and all workspace tests;
7. the physical WiX build, MSI administrative extraction, payload hashes, Burn
   identity and final artifact hashes.

`-SourceOnly` omits only step 7. It does not relax source, contract, format, lint or
test gates.

Windows/Fedora installation scenarios remain physical acceptance evidence and must
not be claimed from source tests alone. Record them directly in
`.AGENTS/EVIDENCE.md`.
