# Validation runbook

This runbook closes only the current three-node MVP. Record results in `../.AGENTS/EVIDENCE.md`; never paste secret values, full environment dumps, raw container inspection, or unredacted logs.

## Candidate identity

Before any run, record:

- source commit and branch;
- SHA-256 and byte size of `QuetzalcoatlSetup.exe` and `Quetzalcoatl.msi`;
- payload manifest SHA-256;
- environment and start/end timestamps.

Do not combine evidence from different installers or an uncommitted source tree.

## Local code and package gate

Run from the repository root:

```powershell
cargo fmt --all -- --check
cargo clippy -p gnx-service -- -D warnings
cargo test --workspace
powershell.exe -NoProfile -ExecutionPolicy Bypass -File installer/build.ps1
Get-FileHash -Algorithm SHA256 target/installer/Quetzalcoatl.msi,target/installer/QuetzalcoatlSetup.exe
```

Also run `sh -n` and `static-check` for `runtime/payload-v1/bin/gnx-pve-cluster-create`, then confirm its SHA-256 equals the entry in `runtime/payload-v1/manifest.json`.

`PrepareWsl` y `ValidateHost` aceptan exclusivamente el exit code Rust `REBOOT_PENDING=14` como `scheduleReboot`; `PrepareWsl` conserva ademÃ¡s el cÃ³digo MSI 3010. El reinicio programado reanuda el preflight; una ejecuciÃ³n Dockur que lo demuestre sÃ³lo aporta evidencia de compatibilidad del instalador y no cierra G5.

## GitHub Actions Dockur compatibility

Use the `codex/ci-dockur` branch of `mayas-alas/windows-rdp-tailscale`. Create a draft release containing the frozen setup asset named `Quetzalcoatl-0.3.0-preview-setup-x64.exe`, then dispatch the workflow with its exact release tag and SHA-256.

The accepted end-user sequence is:

1. Open the tailnet-authenticated HTTPS noVNC URL from the job summary.
2. In `C:\OEM`, run `01-Install-GNX.cmd` and accept the normal UAC flow.
3. Allow real guest reboots; viewer availability alone is not a product result.
4. Run `02-Configure-GNX.cmd` and enter secrets interactively with console echo disabled.
5. Run `03-Collect-GNX-Evidence.cmd` and retain the redacted `gnx-evidence.json` artifact.
6. Exercise `90-Repair-GNX.cmd` and `99-Uninstall-GNX.cmd` only when that lifecycle action is the declared test objective.

For G2, retain the run URL, workflow revision, pinned Dockur digest, installer hash, noVNC screenshot, final `gnx status --json`, service state and binary hashes. A slot that cannot converge because it lacks a real controller is an honest compatibility result, not a fabricated member success.

Dockur slots never close the physical network or cluster gate.

## Three-host functional acceptance

Use three clean, compatible consumer Windows 11 hosts on the same low-latency site. Install the identical frozen setup sequentially:

```text
Host 1 -> controller
Host 2 -> member
Host 3 -> member
```

Before installation, record for all three host pairs a direct Tailscale path, zero packet loss, RTT below 5 ms, synchronized clocks, working MTU, TCP 22/8006 and UDP 5405-5412 reachability inside the GNX runtime. Any DERP/peer-relay path or RTT of 5 ms or more fails G5.

After convergence, retain from each host:

- exact `gnx status --json` with `READY`, the persisted role and the same controller identity;
- PVE cluster view showing exactly one controller, two members, three nodes and quorum;
- `corosync.conf` evidence that each `ring0_addr` is the expected tailnet IPv4;
- absence of an OpenTofu workspace/execution on members;
- absence of Garage/Forgejo duplicates;
- Windows listener inventory proving no PVE port is published on Windows;
- protected state/DPAPI ACL results without blob contents.

Reboot one node at a time: member 1, member 2, then controller. After each reboot wait for `gnx status --json` to return `READY`, reconfirm the same role/controller identities, three-node membership and recovered quorum. A retry may reverify the persisted join; it must never elect, promote or switch controller.

Only this complete evidence bundle closes G5 and raises the functional MVP to 90/100.

## Pilotability gate

G6 additionally requires signed artifacts, capacity preflight, redacted diagnostics, and clean install/resume, upgrade, repair, uninstall and documented recovery results against one frozen candidate. Historical runs remain regression context and do not score the current build.
