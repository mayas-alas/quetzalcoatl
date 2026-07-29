# 0.2.14 delivery evidence

Evidence is recorded only after execution. The superseded unsigned 0.2.13 QA
history belongs in `CHANGELOG.md`; it is not evidence for this cycle.

## Executable gates

| Gate | Command | Result |
|---|---|---|
| Source validation | `powershell -NoProfile -ExecutionPolicy Bypass -File tools/check.ps1 -SourceOnly` | passed, exit 0 |
| Dependency advisories | `tools/security.ps1`, cargo-audit 0.22.2 | passed, 41 dependencies, 0 advisories |
| Installer build and signatures | `installer/build.ps1` | blocked: production certificate absent |
| Unsigned QA build | `installer/build.ps1 -AllowUnsigned` | passed, explicitly non-releasable |
| Self-signed QA build | `installer/build.ps1 -SigningCertificateThumbprint 2DB2BE6CBC78F1CCF5159C8B9359DBD46A9D38AF -AllowSelfSigned` | passed, explicitly non-releasable |
| Production self-signed rejection | same thumbprint without `-AllowSelfSigned` | passed: build failed closed before compilation |

## Legal identity acceptance

| Scenario | Result | Evidence |
|---|---|---|
| Product executable metadata | passed | CLI, tray, service and bootstrap report version 0.2.14, company GNX Labs and joint GNX Labs/Hector AB copyright |
| Setup executable metadata | passed | version 0.2.14, company GNX Labs and exact joint copyright |
| Initial legal disclosure | passed | visual QA shows separate License Agreement and Privacy Policy links before Install can be selected |
| Legal destination | passed | both links are handled by the WiX EulaHyperlink control with `LicenseUrl=https://github.com/mayas-alas/quetzalcoatl/blob/main/LICENSE` |

## Security acceptance

| Scenario | Result | Evidence |
|---|---|---|
| Standard user cannot control installer staging | passed | medium-integrity token cannot inspect or traverse product installer root |
| Reparse point in any staging component fails closed | passed | elevated ignored-by-default Windows test created a directory reparse point, received the expected rejection and removed its fixture |
| Stalled pipe client cannot deny later status | passed | concurrent `gnx status` 25 ms; stalled client forcibly disconnected after five seconds |
| Stop during reconciliation is cooperative | passed | restart, two-second delayed stop and start returned 0; READY restored in 51 seconds |
| Rust dependency advisory scan | passed | cargo-audit 0.22.2 scanned 41 dependencies with zero advisories |
| MSI and Setup Authenticode | blocked | trusted signer and timestamp required |
| Local self-signed Authenticode | passed | four Rust executables, MSI and Setup report Valid for current user; signer CN=GNX Labs and DigiCert timestamp present |
| QA machine publisher trust | passed | public-only certificate appears once in LocalMachine Root and TrustedPublisher; both entries report `HasPrivateKey=False` |
| Elevated Setup publisher evaluation | passed | signed Setup launched through UAC and was closed without starting installation |

## Development certificate

| Property | Evidence |
|---|---|
| Subject | `CN=GNX Labs` |
| Thumbprint | `2DB2BE6CBC78F1CCF5159C8B9359DBD46A9D38AF` |
| Private key | `Cert:\CurrentUser\My`; RSA 3072; CNG export policy `None` |
| Local trust | current-user Root plus public-only copies in `Cert:\LocalMachine\Root` and `Cert:\LocalMachine\TrustedPublisher` |
| Expiration | 2027-07-29 |
| Release status | development only; production mode rejects self-signed certificates |

The previous elevated physical cycle installed the superseded unsigned 0.2.13 QA
candidate only; it is regression context, not 0.2.14 release acceptance:

| Scenario | Result | Evidence |
|---|---|---|
| Upgrade 0.2.12 to 0.2.13 QA | passed | Setup exit 0 in 88.5 seconds |
| Upgrade convergence | passed | same controller, READY/joined/quorate in 14.6 seconds |
| Installer ACL | passed | medium-integrity user receives access denied at both protected root and Installer |
| Programs and Features | passed | one visible 0.2.13 Setup registration and one hidden internal MSI |
| Repair | passed | Setup exit 0 in 41.1 seconds |
| Repair convergence | passed | same controller, READY/joined/quorate in 57.6 seconds |
| Active-reconciliation stop | passed | service restart/stop/start exit 0 in 30.4 seconds |
| Post-stop convergence | passed | same controller, READY/joined/quorate in 51.1 seconds |

The elevated self-signed 0.2.14 QA lifecycle ran through
`tools/qa-lifecycle.ps1`. It is physical development evidence, not public release
acceptance:

| Scenario | Result | Evidence |
|---|---|---|
| Upgrade state | passed | installed 0.2.14 MSI cache matched candidate SHA-256 `533E07377F01DC82D04C4959E161BA2CF7D36503FD4FD1AEB650ABB8BD85B623`; READY on the preserved controller |
| Repair | passed | Setup exit 0 in 39.2 seconds |
| Repair convergence | passed | same controller, READY/joined/quorate 66.7 seconds after repair |
| Repair registration | passed | one GNX Labs 0.2.14 Setup visible and one internal MSI hidden |
| Uninstall | passed | Setup exit 0 in 21.1 seconds |
| Uninstall cleanup | passed | service, both registrations and `Program Files\Quetzalcoatl` absent |
| Fresh install | passed | Setup exit 0 in 25.1 seconds |
| Fresh-install convergence | passed | same controller, READY/joined/quorate in 71.3 seconds |
| Final product surface | passed | service Running/Automatic, one tray process, one visible Setup and one hidden MSI |

## Artifacts

No 0.2.14 artifact is accepted yet. Development-only build:

| Artifact | SHA-256 | Authenticode |
|---|---|---|
| `target/installer/Quetzalcoatl.msi` | `533E07377F01DC82D04C4959E161BA2CF7D36503FD4FD1AEB650ABB8BD85B623` | Valid locally; self-signed GNX Labs; DigiCert timestamp |
| `target/installer/QuetzalcoatlSetup.exe` | `4E02A52A1578CAB71FD72A83649EF81AB3E060A567BE000CA8B45A26EB835318` | Valid locally; self-signed GNX Labs; DigiCert timestamp |
