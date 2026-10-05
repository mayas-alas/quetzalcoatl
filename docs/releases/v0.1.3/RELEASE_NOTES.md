# GnX Node v0.1.3

## Changes

- Adds the installed GnX directory to the Windows host **system PATH**. Open a new host-user terminal and run `gnx-setup` directly.
- Keeps `gnxnodesvc` non-interactive; it does not receive or run a Setup session. Named-pipe authorization still requires an elevated administrator.
- Performs a real versioned upgrade from v0.1.2: Burn detects the prior bundle as `Upgrade`, and MSI detects the prior package as `MajorUpgrade`.
- Makes Burn the single visible Installed Apps entry. Its internal MSI is installed with `ARPSYSTEMCOMPONENT=1`.
- Retains the existing node, distro, CA, Proxmox password, and Tailscale enrollment.

Installing the exact same bundle version remains Windows maintenance/Repair by design. Use the newer numbered bundle for an Upgrade.

## Usage

After upgrading, close old terminals and open a new Administrator terminal:

```powershell
gnx-setup
gnx-setup status
gnx-setup retry
```

Credentials remain accepted only through hidden interactive prompts; never put them in arguments or environment variables.

## Validation

- Upgrade v0.1.2 → v0.1.3: exit code 0.
- Related bundle detection: `Upgrade`; related MSI operation: `MajorUpgrade`.
- CLI from host PATH: `gnx-setup 0.1.3`.
- Visible Installed Apps entries: one, bundle version `0.1.3.0`.
- Rust tests: 54 passed; static installer/runtime/PWA suites passed.
- MSI and Burn build/hash verification passed.
- `GnXHostAgent` running; `app.gnx` and `compute.gnx` HTTP 200.

See `UPGRADE_EVIDENCE.txt` and `evidence/host-path-and-upgrade.png`.

The existing host still reports its previously documented `RUNTIME_FAILED` verifier diagnostic; this upgrade neither hides it nor requests/replaces credentials.
