# Changelog

## 0.2.14

- Establishes GNX Labs as the product manufacturer and credits GNX Labs with
  Hector AB in the product copyright.
- Exposes License Agreement and Privacy Policy links on the initial Setup page;
  both resolve to the repository's canonical AGPL-3.0-only `LICENSE`.
- Adds a fail-closed self-signed Authenticode path for local QA while preserving
  the requirement for a publicly trusted certificate in production.
- Supports explicit public-certificate trust on a controlled QA machine so UAC
  can resolve the self-signed GNX Labs publisher without exposing the private key.
- Adds a reusable elevated QA lifecycle that proves repair, complete uninstall,
  fresh install, ARP shape, tray launch and recovery of the same READY controller.
- Supersedes the unsigned 0.2.13 QA candidate under fresh MSI and Burn identities.

## 0.2.13

- Protects elevated installer state and dependency staging with explicit ACLs,
  complete reparse-point rejection and a locked MSI handle through execution.
- Bounds local IPC with four overlapped instances and per-client timeouts, and
  replaces abrupt service termination with cooperative cancellation and joining.
- Adds pinned RustSec auditing, locked upstream Authenticode policy and mandatory
  signed/timestamped Rust, MSI and Burn release artifacts.

## 0.2.12

- Adds a service-private shutdown event so the WinSW stop helper terminates the
  main service after preserving and stopping its managed Podman Machine.
- Bounds MSI service shutdown instead of leaving WinSW in `StopPending`.
- Supersedes the physically rejected 0.2.11 candidate under fresh identities.

## 0.2.11

- Stops only the dedicated `quetzalcoatl` Podman Machine when the Windows service
  stops, preserving its data while releasing inherited WSL directory handles.
- Runs that closed stop operation under the service identity through WinSW before
  MSI removes product files.
- Supersedes the physically rejected 0.2.10 candidate under fresh identities.

## 0.2.10

- Launches the tray through a checked, short-lived detached operation so Windows
  Installer no longer retains the product directory after installation.
- Removes the unsuccessful post-MSI directory-cleanup package and its transitional
  bootstrap mode.
- Supersedes 0.2.9 under fresh MSI and Burn identities.

## 0.2.9

- Makes cleanup upgrade-aware by retaining a non-empty product tree only when its
  installed `gnx.exe` PE version is newer than the cleanup helper.
- Keeps same-version or unreadable residual payloads fail-closed.
- Supersedes 0.2.8 under fresh identities after physical upgrade analysis.

## 0.2.8

- Runs the tray and supervised service outside the MSI-owned Program Files tree,
  eliminating process current-directory locks during removal.
- Retains the post-MSI empty-directory guard introduced in 0.2.7.
- Supersedes the safely rolled-back 0.2.7 candidate under fresh identities.

## 0.2.7

- Adds a closed Burn cleanup package that runs after MSI uninstall and removes
  only the now-empty product directory.
- Preserves WSL, Podman and durable GNX state while removing every product-owned
  executable, registration, shortcut, PATH entry and package cache.
- Supersedes the physically rejected 0.2.6 candidate under fresh identities.

## 0.2.6

- Closes the tray before MSI file and directory removal, preventing an empty
  `Program Files\Quetzalcoatl` directory after uninstall.
- Adds source enforcement for tray shutdown ordering and complete service removal.
- Supersedes 0.2.5 under fresh MSI product/package and Burn bundle identities.

## 0.2.5

- Makes Setup the sole Programs and Features entry by hiding the chained product
  MSI from ARP.
- Adds source and build regressions that reject any visible internal MSI.
- Preserves 0.2.4 state through the stable MSI and Burn upgrade families under
  fresh product, package and bundle identities.

## 0.2.4

- Recognizes DISM's OEM-code-page output while preserving strict UTF-16LE
  handling and the closed ASCII feature-state marker.
- Supersedes the fully rolled-back 0.2.3 candidate with fresh MSI, package and
  Burn bundle identities.

## 0.2.3

- Ignores pending delete-only temporary-file cleanup while still blocking on
  pending file replacements, CBS servicing and Windows Update reboot markers.
- Supersedes the cached 0.2.2 candidate and the installed 0.2.1 product through
  the stable MSI and Burn upgrade families.

## 0.2.2

- Decodes DISM output as UTF-16LE or UTF-8 before evaluating optional-feature
  state.
- Prevents Setup Repair from requesting another reboot when WSL and Virtual
  Machine Platform are already enabled.
- Retains the 0.2.1 installed-payload validation and recovery contract under new
  MSI package, product and Burn bundle identities.

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

0.2.5 accepts installed 0.1.17, 0.2.0, 0.2.1, 0.2.4 and cached candidate state and installer recovery
checkpoints. Protocol schema 2, persisted-state schema 2, host-profile schema 1,
runtime payload contract 5, the runtime generation and both stable installer
upgrade families remain unchanged.
