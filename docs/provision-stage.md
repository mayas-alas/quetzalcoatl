# Bounded Windows PROVISION

`gnx-setup --provision --bundle <directory> --manifest-sha256 <trusted SHA256> --rootfs <tar> --rootfs-sha256 <trusted SHA256>` runs the elevated, bounded PROVISION stage. All four inputs are mandatory. Hashes must come from the release trust channel; setup cannot establish the provenance of a hash supplied by its caller. `--check` remains non-mutating.

Successful provisioning returns the normal JSON report with `ACTION_REQUIRED`, code `SETUP_PROVISIONED`, and exit 2. This means the PROVISION stage finished, not that a usable runtime or cutover was verified. There is no UI, legacy migration, service start, WSL call, or automatic rollback in this stage.

## Artifacts and boundaries

| Path or object | Purpose |
| --- | --- |
| `C:\ProgramData\GNX-Setup-0.3.1\setup.lock` | Exclusive OS-held setup lock; process exit releases it |
| `C:\ProgramData\GNX-Setup-0.3.1\snapshot.json` | Schema, observed legacy/target presence, trusted manifest/rootfs digests; no credentials or legacy content |
| `C:\ProgramData\GNX-Setup-0.3.1\journal.json` | Last attempted phase and stable failure code; retained after success and failure |
| `C:\ProgramData\GNX-Setup-0.3.1\staged` | Private copies of authenticated manifest, Windows executables, Linux bundle, and rootfs |
| `C:\Program Files\GNX-0.3.1` | New executable directory, separate from legacy GNX; runtime identity gets read/execute only |
| `C:\ProgramData\GNX-0.3.1` | Private runtime root containing `rootfs.tar`, `bundle.tar`, and `operator.sid` |
| `.\gnx-runtime` | Newly created standard local account; generated password goes directly to native account/SCM APIs |
| `GNXRuntime` | Quoted versioned service executable; demand-start and stopped pending the next stage |

The setup directory admits only SYSTEM and Administrators. Runtime data admits SYSTEM, Administrators, and the runtime account. Directories are created with their DACL, rather than populated before protection. Existing setup ACLs are checked; reparse points, preexisting destination directories, preexisting accounts, and preexisting services are refused. Existing legacy directories, account passwords, and service configurations are never overwritten or reconciled.

Manifest schema 1 and exact release `0.3.1` are required. Authentication covers the same bounded manifest bytes that are parsed. Each staged artifact is reauthenticated after copying, preventing source changes between initial validation and copy from becoming trusted installation content. Source handles deny concurrent writes/deletion, copied destinations use create-new semantics, and each artifact is limited to 32 GiB. Manifest input is limited to 1 MiB.

Account rights are granted and read back: service logon plus denied interactive, remote interactive, and network logon. Password buffers and intermediate encodings are zeroized; passwords never enter argv, reports, files, or logs. SCM configuration and stopped status are read back. Recovery schedules 5/15/60-second restarts, then a terminal no-action entry (Windows repeats the final entry); the reset period is one day.

`operator.sid` currently records the elevated setup token's user. Same-user UAC elevation preserves the intended operator; elevation with another administrator's credentials requires a subsequent explicit operator-enrollment step before exposing the broker to the intended operator. PROVISION does not claim to solve that enrollment flow.

## Failure and recovery

Journal phases are `PREFLIGHT`, `STAGING`, `PUBLISHING`, `REGISTERING`, `SECURING`, and `PROVISIONED`. A phase is persisted before its mutation; a failure adds a stable code to that phase. Unknown adapter error text is suppressed in the public report. A journal write failure is reported as `SETUP_FAILURE_JOURNAL_FAILED`, retaining the prior checkpoint.

The snapshot is an inventory of this stage's starting observations, not a backup of legacy data. All targets must be new, so no legacy backup or restoration is needed. Account and SCM creation is transactional for ordinary install failures: a newly created account/service is removed only when the exact versioned GNX service target can be proven. An account-only interruption is retained for inspection because deleting it without a durable ownership witness could remove an unrelated administrator-created account.

Any snapshot or journal blocks a blind retry with `SETUP_RECOVERY_REQUIRED`, including after successful provisioning. Concurrent callers receive `SETUP_BUSY`. An administrator must inspect the protected state and actual account/service/filesystem facts, then either advance a completed stage or explicitly recover the partial installation. Do not merely delete the journal and retry: setup will still refuse existing target account/service/directories. Automated recovery, bootstrap, automatic service startup, verification, and cutover belong to subsequent stages.

## Verification record and remaining gates

Automated Windows GNU tests cover exclusive locking/release, interruption refusal, durable phase replacement, hash tampering, exact manifest version, create-new copy semantics, ACL descriptor construction, report redaction, root/path invariants, honest conflict reporting, success remaining action-required, and mandatory CLI trust inputs. The complete `cargo test --all-targets` suite passes under the installed GNU toolchain, with no host provisioning executed. `git diff --check` passes. The default MSVC build remains unavailable because `link.exe` is not installed; verification uses the installed GNU toolchain.

Clean elevated Windows acceptance is still required for account creation/rights, actual directory ACL propagation and denial to a normal user, SCM registration/configuration readback, interruption at every mutation boundary, account/service conflict preservation, reparse-path refusal, reboot remaining stopped, and unchanged legacy behavior. Those gates must run on a disposable host with approved trusted release inputs. A later stage must explicitly bootstrap and verify GNX before enabling automatic service start or any cutover.

## Exact clean elevated Windows acceptance

Run these steps on a disposable Windows 11 host with WSL 2 enabled, from an
elevated PowerShell opened in the extracted release directory. Do not run them
against a workstation containing legacy GNX data, and do not delete or move
legacy files to make a gate pass.

1. Confirm the baseline without mutation:
   `Get-ChildItem -LiteralPath 'C:\Program Files\GNX','C:\ProgramData\GNX' -Force -ErrorAction SilentlyContinue`
   must return no existing legacy roots. Also confirm that
   `C:\Program Files\GNX-0.3.1`, `C:\ProgramData\GNX-0.3.1`, and
   `C:\ProgramData\GNX-Setup-0.3.1` do not exist.
2. Verify the trusted release manifest and rootfs digests through the approved
   release channel, then run `gnx-setup.exe --check` with all four mandatory
   inputs: `--bundle <trusted-bundle-directory> --manifest-sha256 <trusted-manifest-sha256> --rootfs <trusted-rootfs.tar> --rootfs-sha256 <trusted-rootfs-sha256>`.
   The report must be `ACTION_REQUIRED`/`SETUP_SOURCE_FOUND`, must identify no
   target or legacy conflict, and must say no mutation was performed.
3. Run `gnx-setup.exe --provision` with the same four inputs. Accept only
   `ACTION_REQUIRED`/`SETUP_PROVISIONED`; `READY` is invalid at this stage.
4. Verify that the transaction root contains only protected metadata/staging,
   that `C:\Program Files\GNX-0.3.1` and
   `C:\ProgramData\GNX-0.3.1` contain the staged artifacts, and that
   `operator.sid` equals the elevated operator token SID. Verify ACLs allow
   only SYSTEM/Administrators plus the runtime identity where documented.
5. Verify SCM readback: service `GNXRuntime` is demand-start, stopped, points
   exactly to `C:\Program Files\GNX-0.3.1\gnx-service.exe`, and runs as
   `.\gnx-runtime` according to the host display. Verify
   the configured bounded recovery actions and denied interactive/network logon
   rights for `gnx-runtime`.
6. Verify WSL readback only through the service-owned path: the exact distro is
   `GNX-0.3.1`, its import root is under
   `C:\ProgramData\GNX-0.3.1\wsl`, and the service broker, operator SID,
   runtime data, and distro all reference the same 0.3.1 root. Existing
   `GNX`/`gnx-node` distros are conflicts and must remain untouched.
7. Interrupt a disposable run at each journal phase, then verify a stable
   failure journal and that rerunning returns `SETUP_RECOVERY_REQUIRED` until
   explicit recovery. Test reparse-point refusal and preexisting account/service
   refusal without changing those objects.
8. Reboot the host. Verify the service remains stopped and setup reports the
   required action honestly; only the subsequent, separately approved bootstrap
   and health gate may start WSL or enable service startup.

## Exact lab recovery after a failed provision

On the retained disposable guest, open an elevated PowerShell in the extracted
release directory. Do not run these commands on the host or on a guest with
legacy GNX data. First capture the sanitized setup report and inspect the
journal; never copy passwords, tokens, private keys, update URLs, or full
command output into evidence:

```powershell
Get-Content -LiteralPath 'C:\ProgramData\GNX-Setup-0.3.1\state.json'
Get-Content -LiteralPath 'C:\ProgramData\GNX-Setup-0.3.1\journal.jsonl'
```

If the journal belongs to this exact 0.3.1 target, run the explicit rollback:

```powershell
.\gnx-setup.exe --rollback
```

Rollback removes staged/target artifact files and removes `GNXRuntime` plus
`gnx-runtime` only when SCM proves the service points exactly to
`C:\Program Files\GNX-0.3.1\gnx-service.exe` and runs as `.\gnx-runtime`.
An account-only partial install is retained rather than guessed at; report it
to the lab operator for manual review. Verify unrelated services and accounts
remain present, then archive only the sanitized report and rerun `--check`.
