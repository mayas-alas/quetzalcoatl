# Scope and definition of done — 0.1.15

## In scope

- Stage the pinned WSL and Podman MSIs under a GNX-owned `ProgramData` path before invoking Windows Installer.
- Verify ancillary payload file name, size and SHA-256 before and after staging.
- Persist a bounded install journal across reboot and rerun.
- Preserve verbose dependency MSI logs under `ProgramData\Quetzalcoatl\Installer\logs`.
- Add `gnx -v` and `gnx --version` without contacting `gnx-service`.
- Make member join phases explicit: prepare, authorize, join, verify and confirm.
- Confirm controller/member visibility using the existing typed Fedora agent and PVE cluster state.
- Remove the topology limit based on member count while still requiring exactly one identifiable controller.
- Bump the runtime payload contract only for the new allowlisted confirmation operation.

## Out of scope

- New crates, applications, Windows services or listeners.
- A new controller/member network protocol or enrollment-token service.
- Changes to Named Pipe JSON, protocol schema 2 or persisted-state schema.
- QDevice, HA, automatic node removal or arbitrary cluster administration.
- Docker Desktop, tray UI, OpenTofu or GitHub Actions.

## Definition of done

The source validators pass, no direct Burn WSL/Podman `MsiPackage` remains, the stable staging constants match `dependencies.lock.json`, version flags are local, member phases are persisted, additional members are not rejected by count, and a clean Windows build can still be run through `installer/build.ps1`.
