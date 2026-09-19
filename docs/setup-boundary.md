# Setup presentation and headless boundary

`gnx-setup.exe` owns setup orchestration through `app::setup` and the host port.
The 0.3.1 boundary is headless; a desktop presentation shell is not shipped.
Apply is a finite PRECHECK -> PROVISION -> VERIFY operation and never reports
READY until host verification succeeds. Windows elevation and a reboot may be
required before service/broker/doctor/status verification can complete.
The existing development bootstrap script remains a separate fresh-install tool,
not an upgrade engine or evidence of upgrade readiness.

For a single diagnostic report, invoke `gnx-setup --check`. Existing bundle
arguments retain their strict order:

```text
gnx-setup --check --bundle <directory> --manifest-sha256 <trusted-hash> --rootfs <file> --rootfs-sha256 <trusted-hash>
```

Apply a trusted bundle with the same strict arguments and `gnx-setup --apply`.
Interrupted work requires explicit `gnx-setup --recover` or
`gnx-setup --rollback`; retries preserve the protected journal, snapshot, lock,
and `C:\ProgramData\GNX-Setup-0.3.1\setup-state.json`.

For UI consumption, prefix the same arguments with `--json-progress`. Stdout is
UTF-8 newline-delimited JSON, flushed after each event. Schema 1 emits a `started`
event before checking and a `completed` event after checking; there are no invented
percentages or intermediate migration stages. Both events contain `schema`,
`operation` (`setup-check`), `phase`, `code`, `state`, `exit_code`, and
`apply_available` (always false). The started event has null state and exit code.
The terminal event has `READY`, `FAILED`, or `ACTION_REQUIRED` and exit code
0, 1, or 2 respectively. Its code is `SETUP_CHECK_COMPLETED`,
`SETUP_CHECK_FAILED`, or `SETUP_CHECK_ACTION_REQUIRED`; the initial code is
`SETUP_CHECK_STARTED`. These codes are fixed presentation values, not adapter
error text. The single-report mode supplies diagnostic detail when needed.

Progress is an allowlisted projection: it never forwards arguments, paths,
credentials, raw subprocess output, arbitrary error strings, or report details.
Both transports execute the same preflight exactly once and preserve its outcome.
`READY` means host verification completed. `ACTION_REQUIRED` with
`SETUP_REBOOT_REQUIRED` means provisioning completed but Windows must restart;
it is not a success signal.

`packaging/windows/setup-ui.ps1` is the minimal native-host presentation surface.
It launches `gnx-setup.exe` by absolute path with an argument array, consumes
this stream, and waits for process exit. It treats a missing or malformed
terminal event, an unknown schema, or an exit-code mismatch as a transport
failure. The finite display states are `RUNNING`, `READY`, `ACTION_REQUIRED`,
and `FAILED`; a reboot is never inferred or synthesized. A completed event is
not automatically success. The UI preserves failed/action-required states and
keeps apply disabled; it does not inspect installation paths, hash bundles,
mutate services, invoke installation policy, or duplicate setup logic. There is
no credential input in this protocol.

The Windows build produces and hashes `gnx.exe`, `gnx-service.exe`,
`gnx-setup.exe`, `gnx-linux`, `gnx-linux-bundle.tar`, and `gnx-linux.run` in one
manifest. The thin installer verifies every listed artifact and the rootfs,
rejects known legacy/partial layouts, then forwards provisioning to
`gnx-setup.exe`; it never accepts a service credential. A complete release still
requires the documented Windows/WSL integration and promotion evidence.

The setup host requires an elevated Windows process. Native MSVC builds are
subject to the installed Visual Studio linker/toolchain; if `link.exe` resolves
to a non-MSVC utility, fix PATH or use the documented GNU/LLD toolchain before
claiming a native build. The wrapper does not claim that elevation, account
rights, SCM behavior, WSL, reboot recovery, or post-install health were tested.
