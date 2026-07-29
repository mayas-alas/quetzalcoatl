# Operations

Run `QuetzalcoatlSetup.exe` elevated. It is the sole user-facing interface for fresh
installation, upgrade and repair. It validates the host, verifies pinned
dependencies, handles bounded reboot/resume, installs all internal components and
starts the service and tray.

Setup is the sole Programs and Features entry. The chained MSI is an internal
implementation detail and remains hidden; users must not install, repair or remove
it independently.

Before Windows Installer starts the service, the installed `gnx-service.exe`
validates the runtime lock, every locked runtime file and the pinned Podman Machine
image. A missing or mismatched artifact fails the MSI transaction; Setup must not
report success or launch the tray.

For upgrade, launch the newer Setup. Stable upgrade families locate the installed
product; MSI stops service/tray, replaces keyed files and restarts them after commit.
Schemas 2/2/1, payload contract 5, node identity, role and incomplete member
checkpoints remain compatible. 0.2.14 uses new package identities to replace 0.2.13
while retaining recovery of earlier complete, incomplete and cached maintenance
states. Repair recognizes enabled Windows features without requesting a redundant
reboot. Never delete ProgramData state before an upgrade.

For recovery, use Setup Repair. It repeats closed host/dependency operations and
repairs MSI key paths. A service-only continuation is:

```powershell
gnx restart
gnx status
```

Start troubleshooting with `gnx version`, `gnx status` and `gnx status --json`.
Preserve installer logs, journal, host profile, service logs and JSON status. A
failure before `PROXMOX_READY` must not expose Connect or apply Tailscale Serve.
Malformed or newer state fails closed and must not be hand-edited.

The tray menu contains only status, version and Connect. Connect is enabled only for
a validated `https://gnx-*.ts.net/` PVE URL; localhost, raw IP, alternate port and
non-tailnet URLs are rejected.

Source validation: `.\tools\check.ps1 -SourceOnly`.
Release validation requires a trusted code-signing certificate:
`$env:GNX_SIGNING_CERTIFICATE_THUMBPRINT='<thumbprint>'; .\tools\check.ps1`.
`installer\build.ps1 -AllowUnsigned` is development-only and never produces an
accepted release artifact.

A self-signed QA artifact is equally non-releasable. It requires an explicit
thumbprint plus `-AllowSelfSigned`; the production path rejects that certificate
even when the current user trusts it locally.

`create-development-certificate.ps1 -TrustForLocalMachine` establishes test trust
for UAC in the local-machine Root and TrustedPublisher stores. Use it only on a
controlled QA host; it does not make the publisher publicly trusted.
