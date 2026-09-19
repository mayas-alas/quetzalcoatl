# Persistent Dockur Windows lab

**Date:** 2026-09-18 (America/Mexico_City)  
**Source:** GitHub `main` / `a237fc1` (`persistent-windows-lab` child worktree)

## Current state

The persistent lab was launched in WSL distro `gnx-node` with Podman 4.9.3.
The container and storage volume are intentionally retained for subsequent
rounds:

```text
container: gnx-windows-lab
volume:    gnx-windows-lab-storage
image:     docker.io/dockurr/windows@sha256:0cff9eb0e7aee9953e55bc682852ca4fdca233145a58ae1ec94f0b0c01a2ed30
web:       127.0.0.1:8006 -> 8006
staging:   C:\Users\mayas\AppData\Local\Temp\gnx-windows-lab-staging
WSL stage: /mnt/c/Users/mayas/AppData/Local/Temp/gnx-windows-lab-staging
```

Launch settings included `/dev/kvm`, `/dev/net/tun`, `NET_ADMIN`, 2 CPUs,
`RAM_SIZE=4G`, `CPU_CORES=2`, `/storage`, `/shared`, and `/oem`. No secrets,
legacy mounts, private URLs, or host GNX installation paths were used.

## Release artifacts

Tests passed with 13 passed, 0 failed, and 1 ignored. The MSVC attempt was
not usable because `link.exe` resolved to Git's Unix linker; the available
GNU Rust toolchain was used with the local non-secret `dlltool` support path.

```text
SHA256 46f4c5ef48166d3cd592ec9a7add31078731bbc9a74d490b05bede3436b76e87  gnx.exe
SHA256 c6f28990c91df6889bd86f5524b1398fa90ec23882759e1eb625fbc103f8e461  gnx-service.exe
SHA256 59ac1e6bbb8bbed1e5ca3d16a19cde87fc6cd17b8769cc85670a0a3180286e5e  gnx-setup.exe
```

The staging directory also contains the sanitized `install.bat` hook. Its only
guest command is the absolute path `C:\OEM\gnx-setup.exe --check`; output is
discarded and only a JSON exit-code record is requested on the shared folder.

## Boot and guest-check evidence

Dockur initially received two Microsoft automated-download blocks, then its
static-link fallback completed. Logs show overlay creation, OEM injection,
Windows 11 unattended-image setup, QEMU v11.1.0 boot, and:

```text
Windows started successfully, visit http://127.0.0.1:8006/ to view the screen...
container: running
web loopback: HTTP 200
volume: win11x64.iso, setup.img, data.img, firmware artifacts present
```

**Guest-check gate: BLOCKED / not yet observed.** At documentation time the
container remained running with the Windows QEMU process, but the sanitized
`gnx-setup-check.json` result had not appeared in staging; therefore no guest
exit code or EXE visibility result is claimed. **GNX READY is not claimed.**

## Explicit cleanup (do not run during this persistent round)

```sh
podman rm -f gnx-windows-lab
podman volume rm -f gnx-windows-lab-storage
rm -rf /mnt/c/Users/mayas/AppData/Local/Temp/gnx-windows-lab-staging
```

The cleanup command is recorded for the owner; the container and volume were
not removed by this task.

## Full apply acceptance (2026-09-19)

A complete LAB_ONLY six-artifact bundle and Ubuntu rootfs were supplied through
the retained share. Elevated `--apply` first exposed `ACCOUNT_SID_FAILED`; the
account SID lookup and transactional cleanup were corrected and rebuilt. A
second real apply then returned the expected finite boundary:

```json
{"state":"ACTION_REQUIRED","code":"SETUP_REBOOT_REQUIRED"}
```

After reboot, Windows verified the dedicated `gnx-runtime` account, stopped
`GNXRuntime` service, exact versioned service binary path, and installed
artifacts. Starting the service remained `START_PENDING` because both Windows
features required by WSL were disabled. Enabling `Microsoft-Windows-Subsystem-Linux`
and `VirtualMachinePlatform` succeeded, but the nested Dockur guest then remained
at Windows Boot Manager for more than the bounded observation window. This is a
nested-virtualization acceptance blocker, not GNX `READY`; the persistent volume
is retained for diagnosis and Windows is not downloaded again.

## Retained-lab follow-up poll (2026-09-18)

The existing `gnx-windows-lab` container and `gnx-windows-lab-storage` volume
were not recreated, stopped, or removed. A bounded poll of the existing
staging directory and runtime state found:

```text
container: running
QEMU guest process: running
web loopback: HTTP 200 from the noVNC endpoint
staging: gnx.exe, gnx-service.exe, gnx-setup.exe, install.bat only
guest result: gnx-setup-check.json absent
```

The HTTP 200/noVNC response proves the web endpoint is available, but does not
distinguish a Windows desktop from the installer screen without inspecting the
noVNC pixels. No desktop/installer claim is made from this poll. The exact next
blocker is that the Dockur OEM post-install hook has produced no observable
sanitized result on the shared staging path; the owner must inspect the existing
noVNC screen or continue waiting for `gnx-setup-check.json`, without running any
provision/apply operation.
