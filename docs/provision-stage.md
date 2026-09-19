# Bounded Windows PROVISION

`gnx-setup --provision --bundle <directory> --manifest-sha256 <trusted SHA256> --rootfs <tar> --rootfs-sha256 <trusted SHA256>` runs the elevated, bounded PROVISION stage. All four inputs are mandatory. Hashes must come from the release trust channel; setup cannot establish the provenance of a hash supplied by its caller. `--check` remains non-mutating.

Successful provisioning returns the normal JSON report with `ACTION_REQUIRED`, code `SETUP_PROVISIONED`, and exit 2. This means the PROVISION stage finished, not that a usable runtime or cutover was verified. There is no UI, legacy migration, service start, WSL call, or automatic rollback in this stage.

## Artifacts and boundaries

| Path or object | Purpose |
| --- | --- |
| `C:\Program Files\GNX-Setup\setup.lock` | Exclusive OS-held setup lock; process exit releases it |
| `C:\Program Files\GNX-Setup\snapshot.json` | Schema, observed legacy/target presence, trusted manifest/rootfs digests; no credentials or legacy content |
| `C:\Program Files\GNX-Setup\journal.json` | Last attempted phase and stable failure code; retained after success and failure |
| `C:\Program Files\GNX-Setup\staged` | Private copies of authenticated manifest, Windows executables, Linux bundle, and rootfs |
| `C:\Program Files\GNX-0.3.1` | New executable directory, separate from legacy GNX; runtime identity gets read/execute only |
| `C:\ProgramData\GNX` | Private runtime root containing `rootfs.tar`, `bundle.tar`, and `operator.sid` |
| `.\gnx-runtime` | Newly created standard local account; generated password goes directly to native account/SCM APIs |
| `GNXRuntime` | Quoted versioned service executable; demand-start and stopped pending the next stage |

The setup directory admits only SYSTEM and Administrators. Runtime data admits SYSTEM, Administrators, and the runtime account. Directories are created with their DACL, rather than populated before protection. Existing setup ACLs are checked; reparse points, preexisting destination directories, preexisting accounts, and preexisting services are refused. Existing legacy directories, account passwords, and service configurations are never overwritten or reconciled.

Manifest schema 1 and exact release `0.3.1` are required. Authentication covers the same bounded manifest bytes that are parsed. Each staged artifact is reauthenticated after copying, preventing source changes between initial validation and copy from becoming trusted installation content. Source handles deny concurrent writes/deletion, copied destinations use create-new semantics, and each artifact is limited to 32 GiB. Manifest input is limited to 1 MiB.

Account rights are granted and read back: service logon plus denied interactive, remote interactive, and network logon. Password buffers and intermediate encodings are zeroized; passwords never enter argv, reports, files, or logs. SCM configuration and stopped status are read back. Recovery schedules 5/15/60-second restarts, then a terminal no-action entry (Windows repeats the final entry); the reset period is one day.

`operator.sid` currently records the elevated setup token's user. Same-user UAC elevation preserves the intended operator; elevation with another administrator's credentials requires a subsequent explicit operator-enrollment step before exposing the broker to the intended operator. PROVISION does not claim to solve that enrollment flow.

## Failure and recovery

Journal phases are `PREFLIGHT`, `STAGING`, `PUBLISHING`, `REGISTERING`, `SECURING`, and `PROVISIONED`. A phase is persisted before its mutation; a failure adds a stable code to that phase. Unknown adapter error text is suppressed in the public report. A journal write failure is reported as `SETUP_FAILURE_JOURNAL_FAILED`, retaining the prior checkpoint.

The snapshot is an inventory of this stage's starting observations, not a backup of legacy data. All targets must be new, so no legacy backup or restoration is needed. Account/SCM changes are not transactional: interruption can leave a new account, stopped service, or partial files. Setup intentionally retains those objects and the journal rather than guessing ownership during cleanup or rotating an existing password.

Any snapshot or journal blocks a blind retry with `SETUP_RECOVERY_REQUIRED`, including after successful provisioning. Concurrent callers receive `SETUP_BUSY`. An administrator must inspect the protected state and actual account/service/filesystem facts, then either advance a completed stage or explicitly recover the partial installation. Do not merely delete the journal and retry: setup will still refuse existing target account/service/directories. Automated recovery, bootstrap, automatic service startup, verification, and cutover belong to subsequent stages.

## Verification record and remaining gates

Automated Windows GNU tests cover exclusive locking/release, interruption refusal, durable phase replacement, hash tampering, exact manifest version, create-new copy semantics, ACL descriptor construction, report redaction, success remaining action-required, and mandatory CLI trust inputs. The complete `cargo test` suite passes (27 passed, one existing ignored test), with no host provisioning executed. `git diff --check` passes. The default MSVC build was blocked because PATH resolves `link.exe` to GNU coreutils; verification used the already installed GNU toolchain and Rust LLD with the local `build-support` import library.

Clean elevated Windows acceptance is still required for account creation/rights, actual directory ACL propagation and denial to a normal user, SCM registration/configuration readback, interruption at every mutation boundary, account/service conflict preservation, reparse-path refusal, reboot remaining stopped, and unchanged legacy behavior. Those gates must run on a disposable host with approved trusted release inputs. A later stage must explicitly bootstrap and verify GNX before enabling automatic service start or any cutover.
