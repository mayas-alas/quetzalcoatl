# GnX Node v0.1.21

## Fast WSL memory release

`gnx runtime stop --confirm` now uses the fixed `wsl.exe --shutdown` routine after stopping GnX services. GnX uses a dedicated service identity and treats WSL work for this host operation as disposable. The command can stop all WSL distributions and is intentionally explicit through `--confirm`.

The host keepalive and status monitor remain gated before shutdown, preventing them from reopening the runtime during the transition.

## Installed-host validation

On the elevated test host:

- `runtime stop --confirm` persisted `RuntimeStopped`.
- `vmmemWSL` working set reduced from 2329.5 MiB to 605.5 MiB after 60 seconds.
- `runtime start` returned `NodeReady`.
- `https://app.gnx` returned HTTP 200.
