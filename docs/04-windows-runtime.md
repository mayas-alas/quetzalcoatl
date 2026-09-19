# GNX 0.3.1 Windows runtime boundary

Status: normative  
Audience: Windows engineering, installer, audit  
Last reviewed: 2026-09-19  
Replaces: `windows-runtime.md`, `provision-stage.md`, `setup-boundary.md`, `setup-boundary-validation.md`, Windows parts of lab docs

Windows is a typed bridge into the Linux application core. It is not a second implementation of GNX reconciliation.

## Installed boundary

```mermaid
flowchart TB
  subgraph OperatorSession[Normal operator session]
    Operator[Installing operator SID]
    CLI[gnx.exe: parse request + render JSON]
    Operator --> CLI
  end
  subgraph WindowsHost[Windows host]
    Pipe["\\.\\pipe\\GNX: local-only bounded protocol"]
    SCM[Service Control Manager]
    Service[GNXRuntime / gnx-service.exe]
    Account[.\\gnx-runtime]
    State["C:\\ProgramData\\GNX-0.3.1"]
    Public[public CA export only]
    CLI --> Pipe
    Pipe --> Service
    SCM --> Service
    Account --> Service
    Service --> State
  end
  subgraph WideLinux[Wide Linux distro GNX-0.3.1 owned by gnx-runtime]
    LinuxGNX[/usr/local/bin/gnx]
    Config[/etc/gnx/gnx.toml]
    Runtime[systemd + Podman + GNX runtime]
    Secrets[/var/lib/gnx]
    LinuxGNX --> Runtime
    Config --> LinuxGNX
    Runtime --> Secrets
  end
  Service -->|fixed argv + stdin frames| LinuxGNX
  LinuxGNX -->|sanitized JSON + exit code| Service
```

The pipe allows SYSTEM, Administrators and the exact installing operator SID. Remote pipe clients are rejected. SYSTEM and local Administrators remain trusted principals.

## Setup stages

### `--check`

Non-mutating. Validates inputs, host conflicts, the sealed manifest, its detached Ed25519 signature and artifact/rootfs digests. Expected successful state is `ACTION_REQUIRED` with a source-found/proceed code, not `READY`.

### `--provision`

Elevated, bounded stage. Mandatory inputs:

```text
gnx-setup.exe --provision \
  --bundle <directory> \
  --manifest-sha256 <trusted SHA256> \
  --rootfs <tar> \
  --rootfs-sha256 <trusted SHA256>
```

The manifest must be `sealed` and `manifest.sig` must verify against the public trust root compiled into `gnx-setup`. Artifact hashes are taken from that signed manifest. The rootfs hash must match the `rootfs_sha256` field in that signed manifest and the bytes supplied to setup.

Successful provisioning returns `ACTION_REQUIRED`/`SETUP_PROVISIONED` and exit 2. It means files/account/service were staged, not that runtime is usable.

### Later bootstrap/apply

A separate stage must import/configure the Wide Linux layer, install the verified Linux bundle, start or enable `GNXRuntime` only when approved, and run `doctor`/health checks before any `READY` claim.

## Provisioned objects

| Object | Rule |
| --- | --- |
| `C:\ProgramData\GNX-Setup-0.3.1\setup.lock` | exclusive OS-held setup lock |
| `snapshot.json` | sanitized starting inventory and trusted digests |
| `journal.json` / journal stream | durable phase and stable failure code |
| `staged` | private copies of the authenticated manifest, `manifest.sig`, artifacts and rootfs |
| `C:\Program Files\GNX-0.3.1` | versioned executable directory |
| `C:\ProgramData\GNX-0.3.1` | protected runtime root |
| `operator.sid` | elevated setup token user; alternate-admin elevation requires later explicit enrollment |
| `gnx-runtime` | standard local account; generated password goes only to native account/SCM APIs |
| `GNXRuntime` | quoted exact service path; bounded restart recovery |

Directories are created with final DACLs before population. Reparse points, preexisting targets, preexisting account/service and legacy roots are refused unless an explicit recovery flow proves ownership.

## Account, ACL and broker rules

- The runtime account has service-logon right and explicit deny for interactive, remote-interactive and network logon.
- Password buffers are zeroized and never enter argv, files, reports or logs.
- Runtime data admits SYSTEM, Administrators and the runtime account only.
- The broker protocol has bounded frames and opcodes only for `PLAN`, `APPLY`, `STATUS`, `DOCTOR` and the dedicated secret-response frame.
- The service invokes only `/usr/local/bin/gnx` inside `GNX-0.3.1` with fixed config path and allowlisted operation.

## Wide Linux rules

- Distro name is fixed: `GNX-0.3.1`.
- Existing `GNX`, `gnx-node` or user-created distros are conflicts, not migration sources.
- Automount and Windows interop are disabled.
- The verified Linux bundle crosses the boundary through a controlled service path and is installed root-owned.
- Bootstrap copies are removed after successful installation.
- On Windows, the current implementation adapter is WSL2; this is an internal host
  mechanism, not the product vocabulary or a second runtime.

## Failure and recovery

The [Windows lifecycle checklist](../packaging/windows/uninstall-checklist.md) defines phase exit criteria, ownership witnesses, failure/retry decisions and independent residual checks. Its removal classifications `DRAINING`, `UNREGISTERING`, `CLEANING`, `REMOVED`, `BLOCKED` and `RECOVERY_REQUIRED` are acceptance vocabulary; they are not a claim that every candidate emits those states. Keep lifecycle phase, public JSON state, stable code and process exit as separate evidence fields.

Provision journal phases are `PREFLIGHT`, `STAGING`, `PUBLISHING`, `REGISTERING`, `SECURING` and `PROVISIONED`. A phase is persisted before its mutation. Any journal blocks blind retry with `SETUP_RECOVERY_REQUIRED` until inspected.

Rollback may remove the service/account/files only when ownership is proven. Account-only interruption requires a valid transaction journal at REGISTERING or later plus the account ownership guards; ambiguous ownership is retained for review. Earlier phases cannot authorize account deletion. A rollback result does not imply full uninstall: independently inspect setup-state/lock, roots and registrations before asserting REMOVED or permitting a fresh install.

## Clean elevated acceptance checklist

On a disposable Windows 11 host:

1. Confirm no active legacy or target roots exist.
2. Verify trusted release manifest/rootfs digests.
3. Run `--check`; require non-mutating `ACTION_REQUIRED`.
4. Run `--provision`; require `ACTION_REQUIRED SETUP_PROVISIONED`.
5. Verify protected setup/runtime directories and `operator.sid`.
6. Verify SCM: `GNXRuntime`, exact path, runtime account, stopped/demand-start or configured state expected for the stage, bounded recovery actions.
7. Verify account rights and denied logons.
8. Verify Wide Linux ownership/import path when bootstrap stage runs.
9. Test interruption, conflict, reparse refusal and reboot behavior.
10. Only a later runtime health gate may report `READY`.
