# GnX Node installer

Installer assets live with the product workspace. `scripts/build.ps1` builds both release executables, stages the host and node payloads (including the PWA), then invokes WiX from this directory.

The MSI is the package boundary: it copies files and registers `GnXHostAgent`; it does not run Linux provisioning. `bundle/Bundle.wixproj` is the Burn bootstrapper boundary and offers the installed Rust console Setup as its elevated post-transaction Launch action. Build the MSI first, then the bundle:

```powershell
./installer/scripts/build.ps1
./installer/scripts/verify.ps1
```

The deterministic final artifacts are `build/installer/GnxNode.msi` and
`build/installer/GnxNodeSetup.exe`; the verification script checks both files,
their non-zero sizes, and SHA-256 hashes. To compile Burn independently after
an MSI build, run `dotnet build installer/bundle/Bundle.wixproj -p:Configuration=release`.

Setup is a minimal elevated Rust console and needs no browser runtime or packaged web assets. Upgrades remove the obsolete `gnx-setup.exe.WebView2` profile left by older Setup builds. The MSI appends `[INSTALLFOLDER]` to the Windows **system PATH**, so a newly opened host-user terminal can run `gnx`; `gnxnodesvc` remains a non-interactive service identity and never receives a Setup session. Its default workflow completes the Proxmox password step and then the Tailscale key step through the existing administrator-only named pipe, prompting only when required; `status`, `set password`, `retry`, `runtime stop --confirm`, `runtime start`, `cancel`, `help`, and `version` are safe operational commands. `gnx set password` securely defines or rotates the Proxmox `root@pam` password on new and enrolled nodes without replacing the mesh identity. Re-running after an upgrade preserves the existing node and asks for credentials only when enrollment is needed; silent installs do not prompt or launch interactive onboarding. For a direct MSI install, open a new elevated terminal and run `gnx`; before reopening the terminal, use `Start-Process -Verb RunAs -Wait -FilePath (Join-Path ${env:ProgramFiles(x86)} 'GnX Node\\gnx.exe')`. Add `--pause` when an externally spawned console must remain visible.

Onboarding credentials are accepted only by the five allowlisted control
operations. Setup writes a one-shot bundle under the interactive user's
temporary directory and sends only its bounded path over the administrator-only
named pipe; the host consumes and unlinks the bundle and stages separate
`TS_AUTHKEY`/`PROXMOX_PASSWORD` files for the fixed `install.run` contract.
Neither value is placed in argv, progress, logs, or durable state. The host
creates the product private CA under its protected state directory, installs only
the public certificate into local Windows trust, and publishes loopback-only host
access before reporting ready. It reports the public certificate path plus explicit
Split DNS and CA-trust actions only for external tailnet clients. The named pipe remains the sole secret boundary; credentials are never sent through the PWA.

## Release publication and retention

After live installed-host validation, write `releases/v<VERSION>/RELEASE_NOTES.md`
and run:

```powershell
./installer/scripts/verify.ps1 -PublishRelease
```

This verifies the sealed input and MSI version, copies the current MSI/Burn,
writes their SHA-256 manifest, archives older Markdown under `docs/releases/`,
and removes older release directories. Only the latest release remains under
`releases/`; `CHANGELOG.md` indexes historical notes. Published artifacts of the same version are immutable.
Publish the verified Burn (with its exact embedded MSI), SHA256SUMS and release
notes under the latest `releases/v*/`. Prefer the Quetzalcoatl Hugging Face Space
for the binary when quota allows. If historical LFS exceeds its limit, publish
in the existing `gnx-labs-mx/gnx-node-releases` dataset and link the verified
immutable revision from Space notes; never purge history automatically.
Use the host user's authenticated WSL/Hugging Face account, never the dedicated
runtime identity. Do not upload `build/`, `target/`, intermediates, private state
or credentials, and do not rewrite Git/LFS history. Use
`./installer/scripts/verify.ps1 -PublishRelease -BundleOnly`: both outputs are
verified and the generated local release contains only the complete Burn bundle
and its hash, not a duplicate MSI. Upload the artifact before updating the Space;
check the remote SHA-256 and size. Space contains source, notes and hashes;
its installer is either in the same revision or linked at the immutable
artifact-dataset revision. The previous visible release directory is removed.

## Managed runtime stop and start

To release RAM while preserving the GnX enrollment, state, credentials, and WSL image, run from an elevated terminal:

```powershell
gnx runtime stop --confirm
# equivalent alias: gnx shutdown --confirm
```

The host agent first gates its keepalive and status monitor, stops only its allowlisted GnX services, then executes the fixed `wsl.exe --shutdown` routine to promptly release the shared WSL VM memory. GnX owns the dedicated service identity and treats WSL work under this operation as disposable; this command intentionally stops every running WSL distribution. This prevents a new GnX client from racing the shutdown before `RuntimeStopped` is durably saved. While stopped, `gnx status` reports `RuntimeStopped` and the host agent deliberately does not hold its WSL keepalive open.

The managed account's `.wslconfig` also enables page reporting and gradual automatic memory reclaim while the runtime is active. Those settings reduce idle memory pressure but are not treated as proof of a stop; only successful termination of the fixed distro completes `runtime stop`.

Resume the node with:

```powershell
gnx runtime start
```

Start verifies the fixed distro and allowlisted services before reporting `NodeReady`. Do not use `wsl --shutdown` when the intent is only to pause GnX: that Windows-wide command stops every WSL distribution.

## Uninstallation boundary

A real uninstall is destructive and complete. After stopping `GnXHostAgent`, MSI runs
the installed host binary in its fixed `--cleanup` mode as LocalSystem. It removes
the product-owned `gnxnodesvc` profile containing the `gnx-node` WSL registration
and VHDX, then removes the managed account, runtime data, private CA, credentials,
local Windows CA trust and loopback access, GnX `hosts` entries, the legacy
loopback HTTPS port-proxy rule, and finally product state. It then removes
the service and installed files. The DPAPI credential is the ownership marker: a
same-named account on a clean host is never adopted or deleted.

A genuinely fresh installation also runs this cleanup before starting the new service,
so residue left by releases predating complete uninstall cannot expose an old Proxmox
node or LXC through the new package. Setup exposes no interactive `Deprovision` action;
cleanup belongs exclusively to MSI uninstall/fresh-install transactions.

### Complete uninstall acceptance checklist

- `GnXHostAgent` is absent after uninstall.
- Installed Windows files under `GnX Node` are absent.
- The managed WSL node and Linux runtime are absent.
- Product state, CA material, and credentials are absent.
- Product `hosts` entries, local CA trust, and owned loopback port-proxy rule are absent.
- The `gnxnodesvc` account and managed profile are absent.

## Upgrade and reinstallation contract

Install a newer numbered bundle for an upgrade; installing the exact same bundle version is intentionally maintenance/Repair, not a new release. MSI and Burn keep stable, distinct UpgradeCodes while every published release increments both versions. Burn owns the visible Installed Apps entry and chains its MSI with `Visible="no"`, avoiding a second package entry on bundle installs. The MSI performs a major upgrade of the previous MSI product.

A numbered in-place upgrade preserves and reuses the existing `gnx-node` distro,
managed `gnxnodesvc` identity, state, CA, credentials, and runtime data. The old
MSI receives `UPGRADINGPRODUCTCODE`, and the new MSI receives
`WIX_UPGRADE_DETECTED`; both conditions explicitly suppress destructive cleanup.
The host verifies and refreshes the sealed payload and imports only when no managed
distro exists. MSI installation alone does not apply a compute update. Open the
bundle's elevated Setup at completion: a desired/applied mismatch is reported as
`RUNTIME_UPDATE_PENDING` and requires maintenance confirmation. For automation,
run `gnx upgrade --confirm` from an elevated terminal. Plain `gnx retry` refuses a
pending upgrade, and status never pulls images or restarts guests. Silent installs
do not launch Setup or silently restart compute.

Apply downloads the pinned release first, verifies the running image identity,
package absence and absence of license-server hosts overrides, then acknowledges
compute-affecting configuration only after activation. A failed restart remains
pending; an identical Quadlet or cached image cannot hide an old live container.
KVM, CA-validated HTTPS, gateway and workload checks remain required before
`NODE_READY`. Password rotation explicitly recreates compute to read its protected
EnvironmentFile; enrollment is not reset. In contrast, uninstall followed by
reinstall intentionally starts clean.

The WSL image contract is a sealed `payload/host/gnx-node-rootfs.tar` paired
with `payload/host/gnx-node-rootfs.tar.sha256` containing
`<lowercase sha256>  gnx-node-rootfs.tar`. The host verifies this digest before
registering only the fixed `gnx-node` distro, then requires its `gnxnodesvc`
identity and sealed payload marker before it can invoke `install.run`.

The checked-in release image is an Ubuntu Base 24.04.3 amd64 derivative. Its
official source is
`https://cdimage.ubuntu.com/ubuntu-base/releases/24.04.3/release/ubuntu-base-24.04.3-base-amd64.tar.gz`,
with the published checksum listing at
`https://cdimage.ubuntu.com/ubuntu-base/releases/24.04.3/release/SHA256SUMS`.
The source archive's official SHA-256 is
`6bc2cde3930ad088b3bb46fa45279e96d25bc3810f209850ecbe4722711874f9`.
Ubuntu and Canonical trademark/licensing information is published at
`https://ubuntu.com/legal/intellectual-property-policy`; files and packages
retain their upstream copyright and license notices.

The release input is `payload/host/gnx-node-rootfs.tar`, a gzip-compressed tar
accepted by `wsl --import`, with 37,744,448 bytes and SHA-256
`5215c762cd1562b9b6836b01f6e1a2207cee7bae0ee334e50316af72b9a871af`. Its
manifest is checked in beside it and is packaged into the MSI with the image.
Build and verification reject a missing artifact, malformed/non-lowercase
manifest, unexpected release digest, or content mismatch.

To reproduce the preparation (in an amd64 Ubuntu build environment), download
the URL above, verify the official checksum, extract it into an empty rootfs,
and install only the required boot/runtime packages:

```sh
curl -fLO https://cdimage.ubuntu.com/ubuntu-base/releases/24.04.3/release/ubuntu-base-24.04.3-base-amd64.tar.gz
printf '%s  %s\n' '6bc2cde3930ad088b3bb46fa45279e96d25bc3810f209850ecbe4722711874f9' ubuntu-base-24.04.3-base-amd64.tar.gz | sha256sum -c -
mkdir rootfs
tar --numeric-owner -xzf ubuntu-base-24.04.3-base-amd64.tar.gz -C rootfs
mount --bind /dev rootfs/dev
mount --bind /dev/pts rootfs/dev/pts
chroot rootfs env DEBIAN_FRONTEND=noninteractive apt-get update
chroot rootfs env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends systemd systemd-sysv dbus ca-certificates
printf '[boot]\nsystemd=true\n' > rootfs/etc/wsl.conf
chroot rootfs apt-get clean
rm -rf rootfs/var/lib/apt/lists/*
tar --sort=name --numeric-owner --owner=0 --group=0 --mtime='UTC 1970-01-01' -C rootfs -cf - . | gzip -n > payload/host/gnx-node-rootfs.tar
sha256sum payload/host/gnx-node-rootfs.tar > payload/host/gnx-node-rootfs.tar.sha256
```

The final digest must equal the recorded release digest above; do not replace
the checked-in bytes or manifest with an unreviewed rebuild. The build command
validates and copies both host-payload inputs, and `scripts/verify.ps1`
revalidates the same contract before reporting MSI/Burn hashes.
