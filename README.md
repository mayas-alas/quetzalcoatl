# Quetzalcoatl 0.2.41

Quetzalcoatl is a Windows-managed MVP that installs and reconciles a Fedora Podman
Machine containing the Tailscale and Proxmox runtime for controller and member nodes.
It is developed by GNX Labs, with copyright held jointly by GNX Labs and Hector AB.

`QuetzalcoatlSetup.exe` is the sole installation and maintenance interface. The MSI,
bootstrap helper, Windows service, CLI and tray are internal components managed by
Setup; users do not install or coordinate them separately.

## Product surface

- `gnx status [--json]` reads current service state.
- `gnx configure` submits protected setup inputs.
- `gnx configure platform` separately stores the protected, tag-restricted
  Tailscale enrollment input used by platform LXC workloads.
- `gnx forgejo admin show` verifies and displays the bootstrap administrator
  credential to an elevated local administrator.
- `gnx forgejo admin reset --confirm` atomically rotates that credential.
- `gnx restart` restarts the Windows service; persisted identity and member
  checkpoints survive.
- `gnx version`, `gnx --version` and `gnx -V` print the local version.
- The tray menu contains only status, version and **Conectar**. Connect opens only
  the validated PVE HTTPS URL under the configured tailnet.

No localhost UI, listener or additional product port is introduced.

## Source taxonomy

```text
quetzalcoatl/
|-- apps/
|   |-- gnx/                 # CLI and native Windows tray
|   |-- gnx-service/         # reconciliation service
|   `-- gnx-bootstrap/       # host preflight and dependency recovery
|-- crates/
|   `-- gnx-contracts/       # shared typed contracts; not vendor code
|-- runtime/
|   |-- commands/            # installed locked commands
|   |-- configuration/
|   |-- containers/
|   |-- services/
|   `-- operations/          # embedded stdin programs; not installed
|-- installer/               # WiX sources, canonical/derived assets and build modules
|-- release/                 # authoritative release manifest
|-- tests/                   # wire compatibility fixtures
|-- tools/                   # single validation entry point
|-- docs/                    # four authoritative product documents
`-- .AGENTS/                 # one active delivery contract
```

The workspace has exactly four Cargo packages. Schema versions belong in serialized
contracts and migration tests, never in filenames or parallel implementations.

## Build and validation

```powershell
$env:GNX_SIGNING_CERTIFICATE_THUMBPRINT = '<production certificate thumbprint>'
.\tools\check.ps1
```

This validates repository taxonomy, contracts, remote execution, runtime and
installer sources; runs the pinned RustSec audit, format, lint and tests; then
builds, signs and inspects the MSI and `QuetzalcoatlSetup.exe`. Use `-SourceOnly`
while iterating when physical installer artifacts are not required. An unsigned QA
build requires the explicit `installer\build.ps1 -AllowUnsigned` switch and is not
releasable.

A controlled QA build uses `installer\build.ps1 -QaSigning`. The build reuses a
ten-year non-exportable QA root, renews its shorter code-signing leaf when needed
and embeds only both public certificates. Setup installs that pinned trust before
the product payload, so QA operators do not run certificate commands. This profile
is not a publicly trusted release and is removed entirely from production builds.

Start with [docs/README.md](docs/README.md).
