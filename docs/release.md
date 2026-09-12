# GNX release contract

The current candidate is **0.3.2-rc.2**. See [release-validation.md](release-validation.md)
for executed checks and the explicit acceptance decision. The earlier
documentation-only baseline has been superseded by the Rust implementation.

Build on Windows with the pinned Rust Linux-musl target and `rust-lld`; the
resulting Linux core is later executed and tested inside the isolated runtime:

```powershell
./packaging/windows/build.ps1 -Rootfs ./dist/gnx-wsl-rootfs.tar.gz -SigningKey <protected-seed-path>
```

The build tests Windows and Linux, builds the three Windows binaries and Linux
core, bundles runtime assets, records artifact digests in `manifest.json`, and
creates `manifest.json.sig` with the external Ed25519 release authority. The
private seed is never copied into `dist`.
Before packaging, the pipeline runs `gnx-install.exe verify` so the exact public
key embedded in the shipped installer must accept the detached signature.
Versioned release assets include the Windows ZIP, installer, Linux self-extractor,
Linux bundle, manifest and verified WSL rootfs. Development files and runtime
state must never be uploaded.

Install elevated using the manifest digest independently obtained from the
authenticated release page:

```powershell
./gnx-install.exe --manifest-sha256 PUBLISHED_HASH --rootfs ./gnx-wsl-rootfs.tar.gz
```

`packaging/windows/install.ps1` delegates to this same installer. It does not
provision a second legacy service or request a service-account password.

A clean destructive uninstall is available only for installations carrying the
GNX ownership receipt produced after successful bootstrap:

```powershell
./gnx-install.exe uninstall --confirm REMOVE-GNX-AND-DATA
```

The command emits one JSON result, unregisters WSL `GNX` from the owning service
account, and removes only receipt-matched GNX service, account and roots. It does
not preserve identity or Compute data and must not be used as a recovery action.

Authenticated lifecycle operations are explicit:

```powershell
./gnx-install.exe update --manifest-sha256 <newer-published-hash>
./gnx-install.exe rollback --manifest-sha256 <older-published-hash> --confirm ROLLBACK-GNX
```

The signed `release_serial` enforces direction. Update and rollback back up the
current Windows binaries, stage the Linux release under the fixed `GNX` distro,
run `apply` and `status`, verify the Windows broker, and only then commit the new
receipt. Any pre-commit failure must restore both boundaries or report the
distinct `RELEASE_ROLLBACK_FAILED` terminal code.

Linux installation uses `sudo sh gnx-linux.run`. Then run `gnx doctor`,
`gnx plan`, `gnx apply`, and `gnx status` with the intended config path.
Do not replace or remove existing identity or persistent data to make a test pass.

Promotion to an accepted release requires all G0-G6 checks in [poc.md](poc.md),
including clean installation, uninstall/reinstall, dedicated-account ownership,
reboot recovery, remote DNS/TLS and a complete sanitized evidence bundle.
Known gaps cannot be relabeled as successful acceptance.
