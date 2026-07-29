# Validation

`tools/check.ps1` is the sole validation entry point.

Default execution performs:

1. repository taxonomy and four-package inventory;
2. release, IPC, persisted-state and CLI contracts;
3. remote-execution policy;
4. runtime lock, topology and lifecycle order;
5. installer maintenance, key paths, branding and tray contract;
6. pinned RustSec audit plus locked upstream Authenticode policy;
7. `cargo fmt`, Clippy with warnings denied, and all workspace tests;
8. the physical WiX build, MSI administrative extraction, payload hashes, negative
   missing-lock validation, complete installed-payload validation, Burn identity and
   signed/timestamped final artifact verification.

Install the pinned advisory scanner once with:

```powershell
cargo install cargo-audit --version 0.22.2 --locked
```

`-SourceOnly` omits only step 8. It does not relax source, contract, security,
format, lint or
test gates.

Windows/Fedora installation scenarios remain physical acceptance evidence and must
not be claimed from source tests alone. Record them directly in
`.AGENTS/EVIDENCE.md`.

The release build also rejects an installed MSI with the candidate PackageCode but
different bytes. Any material package change requires a new version and release
identity.

A release build requires `GNX_SIGNING_CERTIFICATE_THUMBPRINT` to identify one
currently valid certificate with a private code-signing key. Rust executables are
signed before MSI packaging; MSI is signed before Burn packaging; the detached Burn
engine and final bundle are signed and timestamped. `-AllowUnsigned` exists only for
local installer QA and its artifacts must never be recorded as accepted release
evidence.

For local Authenticode QA, create or reuse the non-exportable GNX Labs development
certificate and pass its thumbprint explicitly. The script trusts only its public
certificate for the current Windows user. `-AllowSelfSigned` remains non-releasable,
and the default production path rejects self-signed certificates:

```powershell
$certificate = .\installer\create-development-certificate.ps1
.\installer\build.ps1 `
    -SigningCertificateThumbprint $certificate.Thumbprint `
    -AllowSelfSigned
```

The default RFC 3161 endpoint is DigiCert's official
`http://timestamp.digicert.com`. RFC 3161 signs the timestamp response itself;
the signing gate requires a trusted timestamper certificate and accepts no other
plain-HTTP endpoint.

On a dedicated QA machine, an administrator may additionally trust the public
certificate for all local users so UAC can resolve the publisher as GNX Labs:

```powershell
Start-Process powershell -Verb RunAs -Wait -ArgumentList @(
    '-NoProfile',
    '-ExecutionPolicy', 'Bypass',
    '-File', (Resolve-Path '.\installer\create-development-certificate.ps1'),
    '-TrustForLocalMachine'
)
```

This adds only the public certificate to `LocalMachine\Root` and
`LocalMachine\TrustedPublisher`. It is appropriate only for controlled QA
machines and does not create public trust on other computers.

After trust is established, `tools\qa-lifecycle.ps1` performs the reusable
physical QA sequence: repair, complete uninstall and fresh install. It requires
the expected version/controller, validates the timestamped GNX Labs signer, checks
one visible Setup plus one hidden MSI registration, and requires the same READY
controller after maintenance.
