# Dockur Windows injection run

## Result

**BLOCKED — GNX READY was not claimed.** The disposable Dockur container was
started successfully with the requested KVM/TUN capabilities and the web UI
returned HTTP 200, but Windows did not boot during the bounded observation
window. The container remained in `Downloading Windows 11`; QEMU was not
present and the ISO `.aria2` sidecar remained, so guest-side visibility could
not be verified.

## Scope and safety

- WSL distro: `gnx-node` (Podman 4.9.3), running as root.
- Source checkout: clean sibling repo path
  `C:\Users\mayas\orca\workspaces\Quetzalcoat\gnx-0.3.1-functional`.
- Only release artifacts were staged: `dist\gnx.exe`,
  `dist\gnx-service.exe`, and `dist\gnx-setup.exe`.
- No `C:\Program Files\GNX`, `C:\ProgramData\GNX`, credentials, or secrets
  were mounted or passed to the container.
- The child worktree `dockur-run-evidence` was created from
  `eb692f3231808e955827d7e6b8230edf9d685eb0` (GitHub `main` commit).

## Commands and evidence

Image acquisition and pinning:

```sh
podman pull docker.io/dockurr/windows:latest
podman image inspect docker.io/dockurr/windows:latest --format '{{index .RepoDigests 0}}'
# docker.io/dockurr/windows@sha256:0cff9eb0e7aee9953e55bc682852ca4fdca233145a58ae1ec94f0b0c01a2ed30
```

Disposable staging and launch (inside `gnx-node`):

```sh
rm -rf /tmp/gnx-dockur-stage
mkdir -m 0777 /tmp/gnx-dockur-stage
cp /mnt/c/Users/mayas/orca/workspaces/Quetzalcoat/gnx-0.3.1-functional/dist/gnx.exe /tmp/gnx-dockur-stage/
cp /mnt/c/Users/mayas/orca/workspaces/Quetzalcoat/gnx-0.3.1-functional/dist/gnx-service.exe /tmp/gnx-dockur-stage/
cp /mnt/c/Users/mayas/orca/workspaces/Quetzalcoat/gnx-0.3.1-functional/dist/gnx-setup.exe /tmp/gnx-dockur-stage/
podman volume create gnx-dockur-storage
podman run -d --name dockur-gnx-evidence --stop-timeout 120 \
  --device /dev/kvm --device /dev/net/tun --cap-add NET_ADMIN \
  -p 18006:8006 -e VERSION=11 -e RAM_SIZE=4G -e CPU_CORES=2 \
  -e DISK_SIZE=64G -e SHORTCUT=Y \
  -v gnx-dockur-storage:/storage -v /tmp/gnx-dockur-stage:/shared \
  docker.io/dockurr/windows@sha256:0cff9eb0e7aee9953e55bc682852ca4fdca233145a58ae1ec94f0b0c01a2ed30
```

Observed evidence:

```text
podman ps: container running; 0.0.0.0:18006->8006/tcp
podman logs: Starting Windows for Podman v6.05; Downloading Windows 11...
podman exec ... ls -l /shared: gnx.exe, gnx-service.exe, gnx-setup.exe visible
podman exec ... pgrep -a qemu: no output
storage/tmp/win11x64.iso: approximately 7.9G plus win11x64.iso.aria2
curl http://127.0.0.1:18006: HTTP 200 loading page
```

The `/shared` listing proves the host-to-container bind mount only. The
documented Dockur shared folder (Windows `Shared`/`Z:`) was not testable
without a running guest, so no guest EXE visibility claim is made.

Cleanup performed after evidence capture:

```sh
podman rm -f dockur-gnx-evidence
podman volume rm -f gnx-dockur-storage
rm -rf /tmp/gnx-dockur-stage
```

Only the disposable container, volume, and staging directory were removed.

## Follow-up run (2026-09-18)

**Dockur guest boot and injection visibility: SUCCESS. GNX READY is not
claimed.** A fresh run reused the pinned digest and completed the Windows 11
download, QEMU boot, and unattended installation within the bounded 20-minute
attempt. The guest console reached the Windows desktop; the documented
`Shared` shortcut opened in File Explorer and visibly listed `gnx.exe`,
`gnx-service.exe`, and `gnx-setup.exe`.

Temporary names used to avoid existing containers and volumes:

```text
container: dockur-gnx-followup-20260918
volume:    gnx-dockur-storage-followup-20260918
staging:   /tmp/gnx-dockur-stage-followup-20260918
web port:  18007 -> 8006
```

The launch command was the prior command with these unique names and port,
still using `VERSION=11`, `RAM_SIZE=4G`, `CPU_CORES=2`, `DISK_SIZE=64G`,
`/dev/kvm`, `/dev/net/tun`, `CAP_NET_ADMIN`, `/storage`, `/shared`, and the
same pinned image digest. Staged file hashes were unchanged:

```text
b7710ba7a6c0adb19dd26d6b51aeb1a42562b55cb986b90611597fcd6f641693  gnx-service.exe
1f3ec5d7a8f90c5f020b9f97635ffdeb2cc32a26e3b9204a5f27881baf148cea  gnx-setup.exe
222dc5f3ea8d1a4e7a4fe45e7c433cb47b083b039277e621f6b27493aeb231a0  gnx.exe
```

Sanitized readiness evidence:

```text
podman inspect: running
podman top: aria2c during ISO acquisition, then qemu-system-x86_64 present
web UI: HTTP 200 (supplemental only, not treated as guest readiness)
Dockur log: ISO completed; overlay/disk creation completed; Windows started successfully
guest console: QEMU monitor reported VM status: running
guest display: Windows 11 setup reached 84%, then “This might take a few minutes”,
  then desktop
guest shared-folder check: File Explorer opened “Shared” and listed all three EXEs
```

The image was not fully exercised as GNX software: no EXE was installed or
executed, so this run proves guest boot and file visibility only.

Cleanup after the follow-up evidence:

```sh
podman rm -f dockur-gnx-followup-20260918
podman volume rm -f gnx-dockur-storage-followup-20260918
rm -rf /tmp/gnx-dockur-stage-followup-20260918
```

Only those uniquely named temporary resources were removed; existing
containers, volumes, legacy roots, host GNX paths, and secrets were untouched.

## OEM command follow-up (2026-09-18)

**BLOCKED at the OEM command-resolution gate; GNX READY is not claimed.** A
new uniquely named guest used the pinned digest, a persistent temporary
`/storage` volume, and `/oem` containing the three release EXEs plus this
minimal non-secret hook:

```bat
@echo off
set "OUT=Z:\gnx-setup-check.result"
gnx-setup.exe --check >nul 2>&1
set "RC=%ERRORLEVEL%"
>"%OUT%" echo exit_code=%RC%
exit /b %RC%
```

The `/shared` bind mount was a separate temporary directory. The hook wrote
only this sanitized result there:

```text
exit_code=9009
```

Dockur logs showed the ISO/static-link retry, `Adding OEM files to image`,
QEMU boot, Windows Boot Manager, and a running guest. The finite blocker is
Windows `9009` (`gnx-setup.exe` was not found from the OEM hook's working
directory); no raw executable output, credentials, host paths, or secrets were
written. This run therefore proves OEM injection and guest command completion
with a command-resolution failure, not a successful GNX check.

Temporary resources were removed after capture:

```sh
podman rm -f dockur-gnx-oem-20260918b
podman volume rm -f gnx-dockur-storage-oem-20260918b
rm -rf /tmp/gnx-dockur-oem-20260918b /tmp/gnx-dockur-shared-20260918b
```

No existing containers, volumes, legacy roots, host GNX paths, or secrets were
touched.

## Corrected absolute OEM command follow-up (2026-09-18)

**Guest command executed: SUCCESS; GNX READY is not claimed.** The prior OEM
9009 was corrected by changing only `install.bat` to use the absolute guest
path `C:\OEM\gnx-setup.exe --check`; no bare executable name or host path was
used. At the coordinator's direction, the already-downloaded ISO in this
attempt's temporary storage volume was reused after the initial download phase;
the Dockur image itself contains no Windows ISO.

Corrected hook:

```bat
@echo off
set "OUT=Z:\gnx-setup-check.json"
C:\OEM\gnx-setup.exe --check >nul 2>&1
set "RC=%ERRORLEVEL%"
>"%OUT%" echo {"command":"C:\OEM\gnx-setup.exe --check","exit_code":%RC%,"state":"completed"}
exit /b %RC%
```

Unique disposable resources were `dockur-gnx-oem-20260918c`,
`gnx-dockur-storage-oem-20260918c`, `/tmp/gnx-dockur-oem-20260918c`, and
`/tmp/gnx-dockur-shared-20260918c` (web port `18009 -> 8006`). The three EXE
hashes matched the prior clean release staging. Sanitized evidence from the
bounded run:

```text
image: docker.io/dockurr/windows@sha256:0cff9eb0e7aee9953e55bc682852ca4fdca233145a58ae1ec94f0b0c01a2ed30
container: running; qemu-system-x86_64 present; Windows started successfully
reused volume artifact: win11x64.iso, data.img, setup.img, firmware files
guest result: {"command":"C:\\OEM\\gnx-setup.exe --check","exit_code":2,"state":"completed"}
```

The JSON was the only file written to the temporary `/shared` directory and
contains no raw command output, credentials, or secrets. Exit code `2` is the
observed result of the requested `--check`; this confirms execution inside the
guest but does not establish installation, provisioning, or GNX readiness.

Cleanup after capture removed only the uniquely named temporary container,
volume, and staging directories:

```sh
podman rm -f dockur-gnx-oem-20260918c
podman volume rm -f gnx-dockur-storage-oem-20260918c
rm -rf /tmp/gnx-dockur-oem-20260918c /tmp/gnx-dockur-shared-20260918c
```
