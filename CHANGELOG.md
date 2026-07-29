# Changelog

## 0.2.1

- Gives the changed MSI and Burn bundle new product, package and bundle identities
  while preserving both stable upgrade families.
- Repairs incomplete 0.2.0 runtime installations through the normal Setup upgrade.
- Validates the installed runtime lock, every locked payload file and the pinned
  Podman Machine image before Windows Installer starts the service.
- Rejects a locally installed package identity when its cached MSI has different
  bytes, preventing another stale Windows Installer cache collision.

## 0.2.0

- Reorganized the four-package workspace into `apps/` and `crates/` with one
  semantic taxonomy.
- Centralized shared IPC, status, host-profile and migration facts in
  `gnx-contracts`.
- Separated service application, domain and infrastructure boundaries without
  changing schema 2, runtime generation `proxmox-cluster-v2` or payload contract 5.
- Made `gnx version` canonical; retained `--version` and `-V`; rejected ambiguous
  `-v`.
- Separated locked installed runtime files from repository-owned embedded
  operations and probes.
- Added explicit Setup install and repair operations, 0.1.17 journal migration,
  MSI upgrade continuity and deterministic restart handling.
- Preserved bounded member recovery, online-controller role selection, shared host
  resources and PVE-before-Serve ordering.
- Added branded Setup/MSI/CLI/tray resources and the status/version/connect tray
  contract.
- Corrected product metadata to AGPL-3.0-only and Hector AB, while retaining
  separate WinSW and WiX third-party licenses.
- Made release EXE, MSI and Setup outputs reproducible from identical source
  inputs.
- Replaced parallel validation scripts and historical active documents with one
  validation entry point and one documentation taxonomy.

## Compatibility baseline

0.2.1 accepts installed 0.1.17 and 0.2.0 state and installer recovery checkpoints. Protocol
schema 2, persisted-state schema 2, host-profile schema 1, runtime payload contract 5,
the runtime generation and both stable installer upgrade families remain unchanged.
