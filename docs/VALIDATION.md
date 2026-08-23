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
currently valid RSA certificate with a private code-signing key whose chain ends in
the Windows `AuthRoot` store. The closed first-party inventory is `gnx-bootstrap`,
`gnx-service`, `gnx`, `gnx-tray` and the WinSW service wrapper. Their signatures and
the 0.2 product versions are verified both before packaging and after extraction
from MSI/Burn. MSI, the detached Burn engine and final Setup are then signed and
timestamped. Burn validation also verifies the signed WiX bootstrapper application
and the pinned WSL/Podman payloads. `-AllowUnsigned` exists only for local installer
QA and its artifacts must never be recorded as accepted release evidence.

For controlled Authenticode QA, select the explicit QA profile:

```powershell
.\installer\build.ps1 -QaSigning
```

The build creates or reuses a non-exportable `GNX Labs QA Root` valid for ten
years, renews its two-year `GNX Labs QA Publisher` leaf when fewer than 120 days
remain and signs every first-party artifact with that leaf. The QA Bundle contains
only hash-locked DER public certificates and runs the elevated native
`prepare-qa-trust` operation before WSL, Podman or the product MSI. That operation
accepts only the two declared files and hashes, replaces their public contexts in
`LocalMachine\Root` and `LocalMachine\TrustedPublisher`, and is idempotent across
install and repair. No private key enters Setup.

The default RFC 3161 endpoint is DigiCert's official
`http://timestamp.digicert.com`. RFC 3161 signs the timestamp response itself;
the signing gate requires a trusted timestamper certificate and accepts no other
plain-HTTP endpoint.

QA operators therefore launch Setup and accept its normal UAC elevation; there is
no separate PowerShell or certificate-store procedure. This local trust does not
make the first launch acceptable to Smart App Control in enforcement mode, because
the bootstrap cannot run until Windows admits Setup. QA images must already have
that policy disabled or be provisioned by the organization. Setup never changes
Smart App Control, SmartScreen or Defender. Production preprocessing excludes the
QA package and certificates, and production still requires Windows `AuthRoot` or
Microsoft Trusted Signing plus physical enforcement tests.

After Setup establishes trust, `tools\qa-lifecycle.ps1` performs the reusable
physical QA sequence: repair, complete uninstall and fresh install. It requires
the expected version/controller, validates the timestamped QA publisher, checks
one visible Setup plus one hidden MSI registration, and requires the same READY
controller after maintenance.
