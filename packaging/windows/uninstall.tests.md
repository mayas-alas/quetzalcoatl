# Windows uninstall test report

Date: 2026-09-19. Scope: `uninstall.ps1` and its direct tests only; no changes to `legacy`.

## Verified gates

Executed under Windows PowerShell 5.1:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File packaging/windows/uninstall.tests.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File packaging/windows/setup-ui.tests.ps1
```

- Direct suite: **31 isolated checks passed**. Functions are loaded from the script AST; the live uninstall entrypoint is never run against this host. Two child-process checks execute the actual entrypoint with a substituted workflow to verify JSON, exit status, and `-WhatIf`.
- Existing packaging/static suite: **passed**.
- The initial invocation without process-scoped `ExecutionPolicy Bypass` was rejected by the host execution policy. No machine policy was changed.
- **Host lifecycle acceptance is NOT RUN.** These tests do not demonstrate actual SCM deletion, WSL deregistration, profile deletion, ACL inheritance, or registry cleanup on a disposable installation.

## Normal, blocked, and retry flows

| Resource | Normal flow | Explicit blocker | Retry behavior |
| --- | --- | --- | --- |
| Service | Exact executable and local dedicated identity; stop, wait up to 30 seconds, requery stopped state and zero PID; delete and poll disappearance | Different binary, arguments or identity; stop/query failure; pending deletion | Missing service is a no-op; retained SID witness permits account cleanup after deletion finishes |
| Dedicated account | Owned service or protected provisioning journal in REGISTERING/SECURING/PROVISIONED proves ownership; persist exact SID before deletion | Description alone, early journal phase, bad witness ACL/schema, SID replacement, service/task references, query failure | SID witness survives interruption; recreated account with another SID blocks; missing account is a no-op |
| Current-user WSL distro | Exact GNX name plus exact versioned WSL storage path; enumerate running distros, terminate if running, verify stopped, unregister, verify absent | Enumeration/native failure, foreign storage, still running or registered | Stopped distro skips terminate; absent distro is a no-op |
| Other-user WSL distro | Inspect the dedicated account's loaded registry hive, or temporarily mount and finally unload its validated offline hive | GNX registration requires that user's context; other distributions forbid deleting the profile; inaccessible hive blocks | Clean up GNX in its registration owner's context, then rerun; the elevated operator's distro list never proves another user's absence |
| Profile | Exact `C:\Users\gnx-runtime` plus witnessed SID, not special, unloaded, plain tree; remove via Win32_UserProfile; verify registration and directory absent | Loaded/special/redirected profile, alternate path, orphan directory or wrong SID | Missing profile is a no-op; lingering directory remains an explicit ownership blocker |
| Autoruns | Exact executable ownership in machine Run/RunOnce, 32-bit machine Run/RunOnce, and current-user Run/RunOnce; delete only matching values | GNX-named or versioned-root-referencing value with ambiguous command | Enumerate again; unrelated values are retained |
| Shortcuts | Traverse common Start Menu and common Desktop without following reparse points; exact shipped GNX executable target | Enumeration/COM errors or reparse point | Enumerate again; unrelated targets are retained |
| Machine PATH | Remove exact versioned root entries, including quoted/trailing-slash forms, then verify absent | Environment write/read failure or matching entry remains | Removing an already absent entry is a no-op |
| Filesystem | Exact allowed roots and all ancestors/descendants checked for reparse points; installation witness required before root deletion | Foreign/no-witness roots, unexpected type/path, link, inaccessible or residual resource | Retain setup witness until other roots are gone; delete setup witness files last; protected empty setup directory supports a final-directory retry |
| Transaction/output | Hold `setup.lock` while cleaning resources; recheck service/account after acquiring it; atomic SID-witness publication; final resource verification | Busy setup, native/provider errors, or any checked residual resource | `BLOCKED` JSON and exit 1; no exception/native text is serialized; rerun after resolving the reported blocker |
| Dry run | `-WhatIf` does not invoke the workflow | None | No resource mutation and no false REMOVED result |

## Direct suite coverage

The 31 checks cover exact service ownership, stopped-state validation, pending deletion/retry, confirmed service deletion, description rejection, journal phases/SID replay, service/task references, query failure, account deletion/retry, WSL termination order, stopped-distro behavior, failed termination, unregister residuals, failed enumeration, UTF-16/NUL normalization, distro storage ownership, cross-user context, offline-hive unload on failure, profile ownership/loading, orphan profile directory, profile deletion/retry, exact autorun commands, root/reparse checks, foreign-root rejection, witness-last cleanup, protected ACLs, child reparse points, registry value ownership, the final residual-resource gate, sanitized JSON/nonzero exit, and dry-run nonexecution.

## Limits requiring host evidence

This is resumable cleanup, not an undo transaction: already removed resources are not recreated after a later blocker. The persistent witness supports continuing safely.

The script deliberately does not obtain another user's credentials or impersonate the runtime account. A GNX distro registered to that account produces `WSL_RUNTIME_CONTEXT_REQUIRED`; it cannot be removed automatically by the operator's `wsl.exe`. No cross-user take-off success is claimed.

Profile junctions, including Windows compatibility junctions, are conservatively blocked by the plain-tree rule. Offline profiles lacking a readable hive also block. Other users' offline autorun hives and per-user Desktop/Start Menu locations are outside the installer footprint handled here. Concurrent privileged changes after validation remain outside the isolation provided by the cooperating setup lock.

For release acceptance, use a disposable elevated Windows host to install, exercise each actual cleanup stage and interruption, rerun twice, inspect the corresponding registrations/filesystem, and record sanitized JSON and exit codes. Do not count the isolated or packaging suites as that gate.
