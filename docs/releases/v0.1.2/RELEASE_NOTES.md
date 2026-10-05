# GnX Node v0.1.2

## Upgrade

- Replaces local WebView2 Setup with the minimal elevated `gnx-setup` console.
- Initial setup asks for the Proxmox password twice, then the Tailscale key, using hidden interactive prompts.
- Existing enrolled nodes keep their identity, distro, CA, password, and Tailscale enrollment.
- Removes old Setup HTML/JS/CSS and the obsolete `gnx-setup.exe.WebView2` profile.
- Keeps `https://app.gnx` as a status/access-only PWA; its old credential form and bridge are removed.

Use `GnxNodeSetup.exe`. Complete the Burn transaction and click **Launch** for guided setup. Silent installation does not prompt. For MSI-only installation, open an Administrator PowerShell and run:

```powershell
& "${env:ProgramFiles(x86)}\GnX Node\gnx-setup.exe"
```

Operational commands: `status`, `retry`, `cancel`, `help`, and `version`. Never pass credentials on the command line.

## Validation on the upgrade host

- Rust tests: 54 passed (`10` protocol, `38` host-agent, `6` Setup CLI).
- Static installer/runtime/PWA suites: passed.
- MSI and Burn builds plus SHA-256 verification: passed.
- Silent upgrade completed; `GnXHostAgent` is running and installed Setup reports `0.1.2`.
- Old Setup UI and WebView2 profile are absent after upgrade.
- `https://app.gnx` and `https://compute.gnx` both returned HTTP 200 after credential-free reconciliation.
- Screenshots under `evidence/` show the CLI read-only status, status-only PWA, and Proxmox endpoint. No credential values were captured.

## Known host diagnostic

This already-enrolled host still reports `RUNTIME_FAILED` because the canonical verifier marks its Podman 6+ check as requiring attention, although the mesh, gateway, compute check, `app.gnx`, and `compute.gnx` are reachable. The release does not hide or rewrite that diagnostic and did not request or replace either credential. A separate runtime repair is required to reach `NODE_READY`.

Clean-machine provisioning, reboot continuation, and non-administrator pipe denial remain VM acceptance gates; this update-host validation is not presented as proof of those scenarios.
