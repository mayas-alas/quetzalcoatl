# GNX release contract

The current candidate is **0.3.2-rc.2**. See [release-validation.md](release-validation.md)
for executed checks and the explicit acceptance decision. The earlier
documentation-only baseline has been superseded by the Rust implementation.

Build on Windows with an Ubuntu 24.04 WSL build environment:

```powershell
./packaging/windows/build.ps1 -Rootfs ./dist/gnx-wsl-rootfs.tar.gz
```

The build tests Windows and Linux, builds the three Windows binaries and Linux
core, bundles runtime assets, and records artifact digests in `manifest.json`.
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

Linux installation uses `sudo sh gnx-linux.run`. Then run `gnx doctor`,
`gnx plan`, `gnx apply`, and `gnx status` with the intended config path.
Do not replace or remove existing identity or persistent data to make a test pass.

Promotion to an accepted release requires all G0-G6 checks in [poc.md](poc.md),
including clean installation, uninstall/reinstall, dedicated-account ownership,
reboot recovery, remote DNS/TLS and a complete sanitized evidence bundle.
Known gaps cannot be relabeled as successful acceptance.
