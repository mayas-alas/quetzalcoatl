# Setup presentation and headless boundary

`gnx-setup.exe` owns setup orchestration through `app::setup` and the host port.
The 0.3.1 boundary is headless and preflight-only; a desktop presentation shell
is not shipped. No apply operation is implemented or advertised by this boundary.
The existing development bootstrap script remains a separate fresh-install tool,
not an upgrade engine or evidence of upgrade readiness.

For a single diagnostic report, invoke `gnx-setup --check`. Existing bundle
arguments retain their strict order:

```text
gnx-setup --check --bundle <directory> --manifest-sha256 <trusted-hash> --rootfs <file> --rootfs-sha256 <trusted-hash>
```

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
`READY` means only that the current observation-based preflight returned ready;
it does not certify full compatibility, successful installation, or apply readiness.

A future UI must launch the packaged executable by absolute path with an argument
array, consume this stream, and wait for process exit. It must treat a missing or
malformed terminal event, an unknown schema, or an exit-code mismatch as a
transport failure. A completed event is not automatically success. The UI must
preserve failed/action-required states and keep apply disabled. It must not inspect
installation paths, hash bundles, mutate services, invoke PowerShell installation,
or duplicate setup policy. There is no credential input in this protocol.

The Windows build already produces and hashes `gnx-setup.exe`. The development
installer now verifies its manifest hash before mutations and installs it beside
`gnx.exe` and `gnx-service.exe`. A complete release still requires the documented
Windows/WSL integration and promotion evidence.
