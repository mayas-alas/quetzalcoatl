# Validation runbook

This runbook closes only the current three-node MVP. Record results in `../.AGENTS/EVIDENCE.md`; never paste secret values, full environment dumps, raw container inspection, or unredacted logs.

## Candidate identity

Before any run, record:

- source commit and branch;
- SHA-256 and byte size of `QuetzalcoatlSetup.exe` and `Quetzalcoatl.msi`;
- payload manifest SHA-256;
- environment and start/end timestamps.

Do not combine evidence from different installers or an uncommitted source tree.

For 0.1.7, the MSI identity is part of the candidate:

```text
ProductVersion = 0.1.7
ProductCode     = {129BD77D-90DE-4992-86AE-F168C930D549}
PackageCode     = {2164425B-7D79-4186-BDED-EF644CCB8804}
UpgradeCode     = {47D5BD44-D061-407B-913B-47D17EC3BEA9}
Burn ID         = {60314D27-47DF-4118-B937-6D1445BAC9D7}
```

`installer/build.ps1` rejects drift across WiX sources, the pinned deterministic extension, Rust manifests, helper CacheIds and the generated MSI/Burn identities.

The accepted packaging gate requires two builds from the same source to be byte-identical. Compare both sizes, SHA-256 values and direct file contents; then use only the hashes recorded in `.AGENTS/EVIDENCE.md` for remote evidence.

## Local code and package gate

Run from the repository root:

```powershell
cargo fmt --all -- --check
cargo clippy -p gnx-service -- -D warnings
cargo test --workspace
powershell.exe -NoProfile -ExecutionPolicy Bypass -File installer/build.ps1
Get-FileHash -Algorithm SHA256 target/installer/Quetzalcoatl.msi,target/installer/QuetzalcoatlSetup.exe
```

Run the installer build twice without changing sources, retain the first pair outside `target/installer`, and compare it directly with the second pair. Both the MSI and EXE must match byte for byte. Each build also extracts Burn, verifies its registration ID and confirms that the embedded MSI equals the generated MSI.

Also run `sh -n` and `static-check` for `runtime/payload-v1/bin/gnx-pve-cluster-create`, then confirm its SHA-256 equals the entry in `runtime/payload-v1/manifest.json`.

For controller OpenTofu changes, additionally verify:

```powershell
& 'C:\Program Files\Git\bin\sh.exe' -n runtime/payload-v1/bin/gnx-opentofu-entrypoint
& 'C:\Program Files\Git\bin\sh.exe' -n runtime/payload-v1/bin/gnx-opentofu-prepare
cargo test -p gnx-service payload_manifest_matches_all_installed_files
```

The installer build must reject an entrypoint that omits
`-parallelism=1`, does not identify `init`, `validate`, and `apply` failures,
reads the OpenTofu journal in reverse order, or retains Podman
`container died/remove` lifecycle noise ahead of the provider diagnostic.

The live controller regression selects Garage and Forgejo together and must
show that the one-shot completes without concurrent Proxmox mutations. Retain
the final `READY` status, VMIDs 200 and 201, both service health probes and a
redacted OpenTofu failure stage if the run does not complete. Never retain the
transient password file, environment, provider process environment or raw
state.

The physical 0.1.7 recovery run is an in-place upgrade from the installed
0.1.6 controller. Do not uninstall or purge 0.1.6 first. Record that
Windows Installer detects the preserved UpgradeCode, replaces ProductCode
`{7E791841-74B0-4663-8993-952D43CD5C63}` with
`{129BD77D-90DE-4992-86AE-F168C930D549}`, retains the same service SID,
role, controller identity and protected state, and resumes the failed
Forgejo credential rotation idempotently.

On the ready physical controller, validate the new controls from an elevated
PowerShell console:

```powershell
gnx configure forgejo
gnx restart
gnx status --json
```

Enter Forgejo credentials only at the masked prompts. Confirm the new account
can sign in over the private HTTPS URL, the prior managed account is prohibited
when its username changed, the service returns to `READY`, and no credential
appears in process arguments, logs, state, Compose, OpenTofu state, or leftover
`/run` files.

The earlier live failures may already have created the requested user without
administrator status. After upgrading, wait for `gnx status` to report
`overall: ready`, `stage: READY`, then repeat `gnx configure forgejo` with the
same username and password. Before `READY`, the expected result is now
`FORGEJO_CONFIGURATION_NOT_READY` or `FORGEJO_CONFIGURATION_BUSY`, without
starting a Podman operation.

`PrepareWsl` y `ValidateHost` aceptan exclusivamente el exit code Rust `REBOOT_PENDING=14` como `forceReboot`; `PrepareWsl` conserva además el código MSI 3010 con ese comportamiento. El reinicio inmediato detiene la cadena antes del siguiente preflight y Burn la reanuda tras Windows; una ejecución Dockur que lo demuestre sólo aporta evidencia de compatibilidad del instalador y no cierra G5.

Los tres binarios Rust de Windows (`gnx-host-preflight.exe`, `gnx-service.exe` y `gnx.exe`) se compilan con CRT estático. El build inspecciona sus import tables y falla si cualquiera depende de `VCRUNTIME`, `MSVCP`, `MSVCR`, `CONCRT`, `VCOMP`, `UCRTBASE` o `api-ms-win-crt-*`.

## GitHub Actions Dockur compatibility

Use the `validation/gnx-dockur-lifecycle` branch of `mayas-alas/windows-rdp-tailscale`. Create a temporary prerelease containing the frozen setup asset named `Quetzalcoatl-0.1.7-setup-x64.exe`, then dispatch the workflow with its exact release tag and SHA-256.

The accepted end-user sequence is:

1. Open the tailnet-authenticated HTTPS noVNC URL from the job summary.
2. In `C:\OEM`, run `01-Install-GNX.cmd` and accept the normal UAC flow.
3. Allow real guest reboots; viewer availability alone is not a product result.
4. Run `02-Configure-GNX.cmd` and enter secrets interactively with console echo disabled.
5. Run `03-Collect-GNX-Evidence.cmd` and retain the redacted `gnx-evidence.json` artifact.
6. Exercise `90-Repair-GNX.cmd` and `99-Uninstall-GNX.cmd` only when that lifecycle action is the declared test objective.

For G2, retain the run URL, workflow revision, pinned Dockur digest, installer hash, noVNC screenshot, final `gnx status --json`, service state and binary hashes. A slot that cannot converge because it lacks a real controller is an honest compatibility result, not a fabricated member success.

Dockur slots never close the physical network or cluster gate.

The 0.1.3 hosted executions also established a narrower platform limit: the GitHub-hosted Dockur guest entered Windows automatic repair when the nested Windows hypervisor started. Recovering with `hypervisorlaunchtype off` was sufficient to finish and verify the installer payload, but it disables the WSL2/Podman hypervisor path and cannot be used as runtime convergence evidence. A green workflow with schema-valid evidence is therefore not equivalent to `gnx status=READY`; the ledger must record both results independently.

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
