# Scope and definition of done — 0.1.14

## In scope

- Keep exactly four Cargo packages.
- Rename the host-preflight directory to match its package identity.
- Split CLI commands and protocol models into focused modules.
- Organize the service into `service`, `ipc`, `secrets`, `state` and `runtime` zones.
- Add a narrow runtime-control facade without changing reconciliation behavior.
- Update Cargo, WiX, release identities, validation and documentation.
- Remove replaced source paths and exclude generated content from the source ZIP.

## Out of scope

- New crates or applications.
- Changes to Named Pipe commands or JSON.
- Changes to state schema, runtime generation or payload version.
- New CLI commands, tray UI, OpenTofu, GitHub Actions or a Fedora daemon.

## Definition of done

Static validators pass, Rust module sets are exact, no legacy paths remain, release identities are unique, and the Windows build can be run from the existing `installer/build.ps1` entry point.
