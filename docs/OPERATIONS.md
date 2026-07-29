# Operations

Run `QuetzalcoatlSetup.exe` elevated. It is the sole user-facing interface for fresh
installation, upgrade and repair. It validates the host, verifies pinned
dependencies, handles bounded reboot/resume, installs all internal components and
starts the service and tray.

For upgrade, launch the newer Setup. Stable upgrade families locate the installed
product; MSI stops service/tray, replaces keyed files and restarts them after commit.
Schemas 2/2/1, payload contract 5, node identity, role and incomplete member
checkpoints remain compatible. Never delete ProgramData state before an upgrade.

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
Release validation: `.\tools\check.ps1`.
