# Implementation checkpoint — 2026-09-08

This is a compiled development slice, not an accepted G0–G6 PoC.

## Delivered

- Rust crate with domain/application/port/adapter boundaries following the target tree.
- `gnx.exe`, `gnx-service.exe`, Linux `gnx`, Linux self-extracting `.run`, and a Linux tar bundle in `dist/`.
- Strict TOML intent, unknown-field rejection, schema checks, identity/subnet/route validation, deterministic intent revisions.
- Four-command JSON envelope and 0/1/2 exit semantics. Invalid arguments do not execute subprocess text.
- Read-only plan, real systemd observation, Linux prerequisite checks, and refusal to apply an unsealed release.
- Application stage/health/promote sequence and protected filesystem candidate storage. These are foundations, not crash-safe runtime rollback.
- Windows SCM executable, dedicated-account installer boundary, local named pipe DACL for SYSTEM/Administrators/installing SID, fixed GNX distro and fixed Linux command, bounded request frame, strict opcode and JSON parsing.
- Service bootstrap imports a supplied rootfs under the service identity, streams the Linux tar bundle, disables WSL interop/automount, and deletes bootstrap files after successful setup.
- Linux installer preserves existing operator config; it installs the development core only.

## Verified in this work session

- `cargo test --locked --all-targets`: 8 integration tests pass on Windows and Linux.
- Windows `cargo clippy --locked --all-targets -- -D warnings`: passes.
- Release compilation on Windows x86_64 and Linux x86_64 (Ubuntu 24.04 build environment).
- Linux plan returns one parseable JSON document; Windows doctor returns BROKER_UNAVAILABLE with exit 2 when not installed.
- Self-extracting payload SHA-256 matches the tar bundle. Installer execution on a clean host was not tested.

The acceptance-named tests use fake ports to test orchestration. They DO NOT establish G0–G6 acceptance.

## Rebuild

From Windows: `./packaging/windows/build.ps1`. Requires Rust in Windows and `$HOME/.cargo/bin/cargo` plus GCC in the selected build WSL distro. The build script tests both platforms and creates the manifest and packages.

Linux smoke test from the repository: `./dist/gnx-linux plan --config gnx.toml`.
Windows smoke test: `./dist/gnx.exe doctor --config gnx.toml`.
Linux development installation: verify the `.run` against a trusted manifest, then `sudo sh ./dist/gnx-linux.run`.

Windows installer requires explicit trusted manifest/rootfs hashes and an existing dedicated `gnx-runtime` PSCredential. The administrator must provision service logon and deny interactive/network/remote logon rights. The current installer does not create or validate those rights automatically. It refuses an existing GNXRuntime service; updates are not implemented.

## Required next work, in execution order

1. Select and authenticate immutable runtime artifacts. `runtime/release.toml` deliberately contains no fabricated digests; `doctor`/`apply` currently return RELEASE_NOT_AUTHENTICATED after host checks.
2. Implement persistent Compute and an authenticated health probe, then Access enrollment and authoritative DNS, then Control TLS. Runtime assets are inactive templates. There are no running capabilities created by this code.
3. Add per-instance locking, crash journal, runtime rollback, retry cleanup and release identity in revision calculation. Current create-new candidates refuse concurrent staging but are not a complete transaction manager.
4. Harden broker transport: total read/write deadlines, reject trailing frames, bound subprocess output while reading, validate response semantics, verify server identity, and test unauthorized/remote clients. The current single-client synchronous pipe can be stalled by an authorized caller.
5. Add the two-phase secret channel with zeroization and canary-leak tests. No secret intake is implemented; do not put credentials in intent. URL validation is a conservative first pass, not a full URL parser.
6. Finish automatic service-account rights, machine-wide WSL prerequisites, post-install binary/ownership/ACL verification, authenticated update rollback and SCM recovery tests. Bootstrap is not yet transactional and the generated manifest is unsigned; the external trusted digest is mandatory for the development Windows installer.
7. Run remote-client G0–G6, reboot, failure injection and parity acceptance. No gates are claimed passed.

Existing host services observed by systemctl are treated as unverified, even if active. Old dist/release.json is historical and is not used by the new installer; dist/manifest.json describes this build.
