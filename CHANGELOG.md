# Changelog

## 0.2.40

- Adds the canonical elevated `gnx forgejo admin show` credential inspection flow.
- Adds the confirmation-gated `gnx forgejo admin reset --confirm` rotation flow.
- Verifies the persisted administrator credential against Forgejo before disclosure,
  rotates it through the local API without placing secrets in argv or environment,
  and serializes reconciliation, deployment and administration through one lock.
- Closes release signing coverage over every first-party executable and its copies
  inside MSI/Burn, checks matching product versions and verifies the signed WiX,
  WSL and Podman payloads before accepting Setup.
- Rejects production signing identities that are self-signed, non-RSA or do not
  chain to Windows `AuthRoot`; local development trust remains QA-only.

## 0.2.39

- Generates Forgejo's OAuth2 JWT secret with the required 32-byte base64url
  encoding and replaces the invalid padded value emitted by 0.2.38.
- Constrains each nested Tailscale interface to the measured safe MTU so HTTPS
  certificates and service-to-service traffic do not stall on fragmented packets.
- Signs the pinned Windows service wrapper as a GNX Labs release input and verifies
  that the MSI contains that exact signed binary.
- Confirms the repository contains no version-suffixed implementations, Ansible
  runtime, security-policy mutation, or unresolved maintenance markers.
- Stages the platform bundle exclusively from `platform.lock.json` and rejects
  empty or unlocked source directories so removed execution layers cannot leak
  back into installer staging.

## 0.2.38

- Persists Forgejo's OAuth2 JWT signing secret as a distinct GNX-owned
  32-byte secret mounted read-only into the container.
- Normalizes runner registration output to its final line and accepts only the
  canonical UUID shape returned by Forgejo's idempotent registration command.

## 0.2.37

- Canonicalizes optional token checkpoints as an all-or-nothing pair.
- Revokes only GNX-managed token names through Forgejo's supported API before
  recreating a missing pair, preventing collisions and orphan credentials.
- Validates each generated access token as exactly 40 lowercase hexadecimal
  characters before it can enter an HTTP header or durable checkpoint.

## 0.2.36

- Checkpoints generated Forgejo API tokens before later bootstrap stages can
  fail, so reconciliation can resume without losing credentials.

## 0.2.35

- Writes the Forgejo runner registration secret without a trailing record
  delimiter, preserving Forgejo's exact 40-character contract.
- Adds a static regression gate for the delimiter-free runner secret file.
- Keeps fresh-install disk allocation strict while allowing upgrade and repair
  to reuse an existing managed-machine allocation.

## 0.2.34

- Makes every LXC host prerequisite fail closed instead of relying on `set -e`
  semantics inside a conditional shell function.
- Requires each LXC bootstrap to emit exactly one `LXC_HOST=ready` completion
  marker before service configuration may proceed.

## 0.2.33

- Preserves both command context and the final diagnostic when remote output
  exceeds the status bound.
- Reports the exact secret-free LXC service stage that failed during platform
  reconciliation.

## 0.2.32

- Removes an incomplete local backend declaration before the initial OpenTofu
  pass and rewrites the canonical remote declaration atomically after Garage.

## 0.2.31

- Emits the generated S3 backend as valid multiline HCL instead of nesting a
  backend block inside a single-line `terraform` block.

## 0.2.30

- Declares the dynamically enabled S3 state backend in canonical `backend.tf`
  and removes the invalid override-form remnant before initialization.

## 0.2.29

- Enrolls Tailscale sidecars through the official declarative `tailscaled`
  configuration with a root-only `file:` auth-key reference.
- Keeps the transient enrollment container until validation completes so a
  bounded failure includes its logs, then removes the container and credentials.

## 0.2.28

- Canonicalizes every persisted platform secret without a trailing record
  delimiter before constructing bounded service input.
- Supersedes 0.2.27 after physical Garage reconciliation exposed a doubled
  newline following persisted S3 access keys.

## 0.2.27

- Isolates every guest-operation helper so POSIX shell variables cannot mutate
  caller paths while copying locked Compose and Tailscale Serve assets.
- Supersedes the physically tested 0.2.26 candidate whose asset destination
  could become `serve/serve/serve.json`.

## 0.2.26

- Removes the Ansible execution layer and preserves OpenTofu solely for PVE
  resources.
- Converges Docker independently in every managed LXC through one locked
  `pct exec ... sh -s` operation, then applies Garage, Forgejo, runner and
  workload Compose definitions in dependency order.
- Restores digest-pinned Tailscale sidecars with one-time file-based enrollment;
  permanent Compose and service repositories receive no enrollment credential.
- Makes the interactive tray launch non-vital so Windows application-control
  policy cannot roll back an otherwise valid system installation.

## 0.2.25

- Applies bounded OpenTofu registry discovery and provider download retries to
  tolerate the observed nested-network DNS failures without changing providers or
  introducing a second cache/mirror architecture.

## 0.2.24

- Makes configuration responses use their own closed stage taxonomy, prevents one
  malformed pipe request from terminating the service, and preserves small
  responses until the client consumes them.
- Enrolls LXC identities through a transient `tailscaled` configuration so the
  auth key remains outside process arguments and is removed after enrollment.
- Supersedes 0.2.23 after physical execution exposed both defects.

## 0.2.23

- Separates PVE API credentials from Ansible transport by generating a persistent,
  root-only Ed25519 key authorized only inside the managed PVE container.
- Supersedes 0.2.22 after physical execution confirmed that PVE correctly rejects
  root password authentication over SSH.

## 0.2.22

- Binds Ansible to the canonical `platform/ansible/roles` directory and validates
  that execution contract before packaging.
- Supersedes 0.2.21 after physical execution reached the foundation playbook and
  exposed the missing role search path.

## 0.2.21

- Accepts both canonical Podman image-ID representations while still enforcing a
  complete lowercase SHA-256 digest before executing the immutable Ansible image.
- Supersedes 0.2.20 after physical reconciliation proved that image construction
  succeeded but its valid digest representation was rejected.

## 0.2.20

- Preserves the final platform failure tail so physical Ansible and Podman errors
  remain actionable through `gnx status`.
- Supersedes the physically incomplete 0.2.19 QA candidate without replacing
  controller, LXC or protected enrollment state.

## 0.2.19

- Restores the missing immutable Ansible execution image and rejects platform
  bundles whose operations reference absent files.
- Correctly enrolls fresh LXC Tailscale identities and removes transient
  enrollment material even when convergence fails.
- Supersedes the incomplete 0.2.18 QA installation under fresh MSI and Burn
  identities while preserving its LXC and controller state.

## 0.2.18

- Adopts compatible legacy Tailscale state while reconciling each service to its
  canonical `gnx-*` hostname, service tag and MagicDNS behavior.
- Supersedes the uninstalled 0.2.17 QA candidate under fresh release identities.

## 0.2.17

- Generates platform secrets from the Fedora base runtime without introducing an
  undeclared OpenSSL dependency.
- Preserves the accepted platform enrollment input and supersedes the physically
  rejected 0.2.16 candidate under fresh identities.

## 0.2.16

- Hides the internal MSI with both Burn visibility and `ARPSYSTEMCOMPONENT`,
  restoring Setup as the sole Programs and Features entry.
- Supersedes the physically rejected 0.2.15 candidate under fresh MSI and Burn
  identities without changing the platform contract.

## 0.2.15

- Adds the reproducible GNX platform foundation: Garage object storage, Forgejo,
  a dedicated Actions runner and private LXC-level Tailscale identities.
- Introduces `gnx configure platform` with a separate DPAPI-protected service
  enrollment key and automatic controller-only reconciliation.
- Packages and verifies a locked platform bundle alongside runtime contract 6
  under the `proxmox-platform` generation.

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
