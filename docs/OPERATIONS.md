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
Schemas 2/2/1, payload contract 6, node identity, role and incomplete member
checkpoints remain compatible. 0.2.40 uses new package identities to replace 0.2.39
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

On a READY controller, configure the foundation enrollment identity once from an
elevated interactive console:

```powershell
gnx configure platform
```

Supply a reusable, preauthorized Tailscale auth key restricted to
`tag:quetzalcoatl-service`. This does not reconfigure the node and does not belong
in a Forgejo repository or `.env`. Expiry prevents future enrollments but does not
invalidate LXC identities already enrolled; replace the protected platform key
through the same command before provisioning another LXC.

Forgejo administrator access is available only from an elevated console on the
active controller:

```powershell
gnx forgejo admin show
gnx forgejo admin reset --confirm
```

`show` verifies the stored password before displaying it. `reset` generates a new
48-character hexadecimal password; it does not accept caller-selected text. Treat
the command output as a secret and clear terminal scrollback after transferring it
to a password manager. Rotation is serialized with reconciliation and deployment.

Nested service sidecars constrain `tailscale0` to MTU 1100 after Compose startup.
Do not raise it until a do-not-fragment probe of at least 1200 payload bytes passes
between the Windows host and every managed service; small pings alone do not prove
that HTTPS certificates can cross the nested path.

Quetzalcoatl never disables Smart App Control, SmartScreen or Defender. A
self-signed QA certificate can still lack cloud reputation even when Authenticode
is valid. Re-enable Smart App Control from Windows Security after a QA exception;
production artifacts require a publicly trusted code-signing identity.

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
even when the current user trusts it locally. Production additionally requires an
RSA publisher chain rooted in Windows `AuthRoot`; the build extracts MSI and Burn
and verifies the signature and product version of every first-party executable,
plus the trusted signatures of the WiX, WSL and Podman binaries they load.

`create-development-certificate.ps1 -TrustForLocalMachine` establishes test trust
for UAC in the local-machine Root and TrustedPublisher stores. Use it only on a
controlled QA host; it does not make the publisher publicly trusted.
