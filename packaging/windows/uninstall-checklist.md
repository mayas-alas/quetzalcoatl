# Windows uninstall evidence checklist

This is the acceptance contract for the GNX 0.3.1 Windows footprint, not proof that a candidate implements every gate.
Run `packaging/windows/setup-ui.tests.ps1` first, then run the candidate `packaging/windows/uninstall.ps1` from elevated PowerShell on a disposable or declared Windows host. A missing script blocks acceptance. Attach sanitized observations; never attach credentials, private keys, update URLs or raw environment/registry dumps.

## Lifecycle taxonomy

These names classify lifecycle evidence. They do not replace the public JSON states `READY`, `ACTION_REQUIRED`, `FAILED`, the UI label `RUNNING`, or existing stable error codes. The setup progress transport emits `started`/`completed`; it does not stream all journal phases.

| Lifecycle state | Meaning and required observation | Allowed next state |
| --- | --- | --- |
| `PREFLIGHT` | Validate trust, elevation, conflicts and ownership before target mutation; record inventory and lock result. | STAGING, DRAINING, BLOCKED |
| `STAGING` | Copy into protected staging and reauthenticate private copies. | PUBLISHING, RECOVERY_REQUIRED |
| `PUBLISHING` | Publish versioned files and operator SID under protected roots. | REGISTERING, RECOVERY_REQUIRED |
| `REGISTERING` | Create the owned runtime account and SCM registration. | SECURING, RECOVERY_REQUIRED |
| `SECURING` | Apply and verify runtime ACLs and account restrictions. | PROVISIONED, RECOVERY_REQUIRED |
| `PROVISIONED` | Provisioning finished; require ACTION_REQUIRED / SETUP_PROVISIONED / exit 2, then independent runtime bootstrap and health gates. | DRAINING, RECOVERY_REQUIRED |
| `DRAINING` | Stop owned service/processes and owned WSL activity; verify bounded termination before deletion. | UNREGISTERING, RECOVERY_REQUIRED |
| `UNREGISTERING` | Remove only proven owned service, distro, account and shell registrations. | CLEANING, RECOVERY_REQUIRED |
| `CLEANING` | Remove owned files and staging after unregistering; independently enumerate residuals. | REMOVED, RECOVERY_REQUIRED |
| `REMOVED` | Every in-scope absence check succeeds; record preserved foreign objects separately. | PREFLIGHT |
| `BLOCKED` | Preconditions or ownership cannot be established; no destructive mutation is authorized. | PREFLIGHT after resolving blocker |
| `RECOVERY_REQUIRED` | Mutation may have occurred, state is incomplete, or postconditions are unverifiable. Preserve last phase and stable failure code. | Explicit inspected recovery only |

The first six names are provisioning journal phases in `src/adapter/windows/setup.rs`. Removal phases and BLOCKED/RECOVERY_REQUIRED are acceptance classifications unless the candidate explicitly emits them. Never fabricate journal events for phases inferred from host observations. A provision failure after setup metadata exists can require recovery even when target mutation has not started.

## Ownership rules

| Object | Required witness before destructive action | Conflict handling |
| --- | --- | --- |
| Program/data/setup roots | Exact canonical versioned paths, protected ancestors, no ReparsePoint anywhere in the tree, candidate/transaction inventory and expected ACLs. | Path name alone is insufficient; retain foreign content or report blocked. |
| `GNXRuntime` | SCM PathName resolves to the exact owned `gnx-service.exe`; StartName resolves to the expected runtime SID; compare registration with inventory. | A service name alone grants no ownership. |
| `gnx-runtime` | SID matches protected uninstall-owner metadata or a valid transaction/SCM witness; inspect other services, tasks and registrations referencing it. Description alone grants no ownership and native setup may leave it empty. | Do not delete a borrowed account or an account with foreign references. |
| WSL `GNX-0.3.1` | Registration owner SID, exact BasePath and distribution identity match the runtime account and owned data root. | Operator-session `wsl --list` is insufficient; a namesake under another SID is foreign. |
| Shortcuts, machine/user Path and Run | Exact destination/command and registry scope match the owned executable; compare against pre-run inventory. | Preserve unrelated values, entries and shortcuts; report conflicting GNX registrations. |

Legacy roots, distributions and historical `legacy` sources are never adoption or cleanup targets. Never infer ownership from a display name, a substring, or a successful delete command.

## Failure and retry matrix

| Injection / starting condition | Expected classification and evidence | Retry rule |
| --- | --- | --- |
| Bad manifest/rootfs hash, no elevation, foreign namesake or reparse point | BLOCKED; non-success exit and unchanged target/foreign inventory. | Correct precondition, repeat preflight; inspect any created setup metadata. |
| Concurrent setup/removal or held setup lock | BLOCKED; no competing mutations. | Retry only after proving the other operation ended. |
| Interrupt STAGING or PUBLISHING; disk full/access denied | RECOVERY_REQUIRED; last attempted phase, stable code and retained partial inventory. | Inspect journal/snapshot; explicit recovery, never blind reinstall. |
| Interrupt REGISTERING or SECURING, including account-only creation | RECOVERY_REQUIRED; correlate journal, account SID, SCM and ACLs. | Prove ownership before rollback; retain ambiguous account. |
| Service/WSL stop timeout during DRAINING | RECOVERY_REQUIRED if mutation began; no REMOVED claim. | Resolve held resource and revalidate ownership before retry. |
| Foreign account reference or WSL owner mismatch | BLOCKED before mutation, otherwise RECOVERY_REQUIRED; foreign objects unchanged. | Operator resolves conflict; never broaden deletion scope. |
| Denied delete, locked file, failed native command during UNREGISTERING/CLEANING | RECOVERY_REQUIRED; record residual object class and native exit, never success-only counters. | Resume only through an implementation-supported path after fresh inventory. |
| Journal corrupt/missing after partial mutation, interrupted rollback or reboot | RECOVERY_REQUIRED; absence of evidence is not evidence of ownership. | Manual inspected recovery; preserve unknown objects. |
| Repeat uninstall on an already clean host | REMOVED only if all absence checks succeed; counters may be zero. | If candidate cannot handle absent objects, record failed idempotence gate. |
| Reinstall after removal | PREFLIGHT then PROVISIONED only with a fresh transaction and expected ownership. | Residual locks, registrations or roots fail the removal gate. |

## Evidence procedure

Record candidate commit, artifact hashes, Windows/PowerShell/WSL versions, elevation and observer SID, expected ownership, pre/post inventory, exit code, reported result, last known phase, and PASS/FAIL/BLOCKED per check. Store evidence outside roots being removed. Use allowlisted fields and counts rather than raw secrets or command lines.

1. Snapshot the three versioned roots, exact SCM/account identities, per-owner WSL registrations, shortcut targets, and matching Path/Run entries. Query failures must be recorded as failures, not empty inventories.
2. Execute each failure injection above on a separately reset disposable host. Capture the immediate state and a second observation after retry/reboot where applicable. Never inject faults into the developer host.
3. Run uninstall, capture its exit code immediately, and perform every absence check below from an independent observer. A `result: REMOVED` string, zero exit, or removal counters alone cannot pass acceptance.
4. Repeat the checks after reboot; verify no process, task or autorun recreates owned state. Then run repeat-uninstall and fresh-install cases. Compare foreign sentinels before/after; any modification fails the gate.

## Required evidence

1. The script reports `result: REMOVED`; record whether `service_removed`, `distro_removed`, `account_removed`, and `shortcuts_removed` match the host state.
2. Verify `Get-Service GNXRuntime -ErrorAction SilentlyContinue` returns no service.
3. Verify `Get-LocalUser gnx-runtime -ErrorAction SilentlyContinue` returns no account, or capture the script failure showing a non-GNX owner/reference blocker.
4. Verify `Test-Path -LiteralPath 'C:\Program Files\GNX-0.3.1'`, `Test-Path -LiteralPath 'C:\ProgramData\GNX-0.3.1'`, and `Test-Path -LiteralPath 'C:\ProgramData\GNX-Setup-0.3.1'` are all false.
5. Verify `wsl.exe --list --verbose` in the registration owner's context no longer lists the fixed distro `GNX-0.3.1`; independently inspect the recorded owner SID's WSL registration and BasePath. An unavailable owner context is BLOCKED, not an empty list.
6. Verify the machine `Path` no longer contains `C:\Program Files\GNX-0.3.1` and no `gnx.exe` shadows remain in `Get-Command gnx.exe -All`.
7. Verify no owned GNX autorun remains under `HKLM:\Software\Microsoft\Windows\CurrentVersion\Run`, the applicable 32-bit view, or the installing/runtime users' Run keys. Inventory foreign/legacy entries without deleting them.
8. Enumerate common and per-user Start Menu/Desktop shortcuts by resolved target; verify no shortcut, scheduled task or process references the removed executable roots. Do not remove unrelated `gnx.exe` commands merely because they share a name.
9. Confirm no owned setup lock, staged payload, journal or snapshot remains under `C:\ProgramData\GNX-Setup-0.3.1`; externally retained sanitized evidence is intentional and must be listed.
10. Cross-check all `*_removed` counters against the initial inventory and final observations. Any residue, failed query, contradictory count, or reboot recreation fails REMOVED even when the script returned success.

## Gaps

- No GNX-owned tray app exists in the current architecture, so there is no tray process to stop or unregister.
- The uninstaller must remove only roots and registrations with exact GNX 0.3.1 ownership witnesses; any conflict is a blocker, not an adoption path.
- Static source checks cannot prove ACL effectiveness, WSL owner context, termination, idempotence or absence of residue. Clean-host, interruption and reboot evidence is required separately.
- Existing UI checks do not establish ordered/unique started-completed events or operation-to-mode matching; malformed/reordered/duplicate/mismatched fixtures must fail before transport acceptance can pass.
- The current uninstaller emits BLOCKED for all caught failures, including failures after mutation. Classify those runs as RECOVERY_REQUIRED in acceptance evidence and preserve the native code; removal phases are not durably journaled.
- Runtime-owned WSL can require an owner context unavailable to the elevated operator; current refusal codes WSL_RUNTIME_CONTEXT_REQUIRED / WSL_RUNTIME_CONTEXT_UNVERIFIED are honest blockers, not complete removal support.
- Current shell cleanup covers common shortcuts, machine Path, and HKLM/HKCU autoruns. Per-user shortcuts, other users' Run keys and user Path still require independent residual checks.
- Plain-tree checks establish path safety, but recursive deletion of versioned roots does not prove ownership of every child. Inject a foreign sentinel and require preservation/refusal; static checks alone cannot certify this gate.
