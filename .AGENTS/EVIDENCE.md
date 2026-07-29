# 0.2.13 delivery evidence

Evidence is recorded only after execution. The accepted 0.2.12 history belongs in
`CHANGELOG.md`; it is not evidence for this security cycle.

## Executable gates

| Gate | Command | Result |
|---|---|---|
| Source validation | `powershell -NoProfile -ExecutionPolicy Bypass -File tools/check.ps1 -SourceOnly` | passed, exit 0 |
| Dependency advisories | `tools/security.ps1`, cargo-audit 0.22.2 | passed, 41 dependencies, 0 advisories |
| Installer build and signatures | `installer/build.ps1` | blocked: production certificate absent |
| Unsigned QA build | `installer/build.ps1 -AllowUnsigned` | passed, explicitly non-releasable |

## Security acceptance

| Scenario | Result | Evidence |
|---|---|---|
| Standard user cannot control installer staging | passed | medium-integrity token cannot inspect or traverse product installer root |
| Reparse point in any staging component fails closed | passed | elevated ignored-by-default Windows test created a directory reparse point, received the expected rejection and removed its fixture |
| Stalled pipe client cannot deny later status | passed | concurrent `gnx status` 25 ms; stalled client forcibly disconnected after five seconds |
| Stop during reconciliation is cooperative | passed | restart, two-second delayed stop and start returned 0; READY restored in 51 seconds |
| Rust dependency advisory scan | passed | cargo-audit 0.22.2 scanned 41 dependencies with zero advisories |
| MSI and Setup Authenticode | blocked | trusted signer and timestamp required |

The elevated physical cycle installed the unsigned QA candidate only; it is not
release acceptance:

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

## Artifacts

No 0.2.13 artifact is accepted yet. Development-only build:

| Artifact | SHA-256 | Authenticode |
|---|---|---|
| `target/installer/Quetzalcoatl.msi` | `C82A5D3B4F6FAB56C7F8928FCA31D4311B1373695CC5F8E34C94C6C17CC1CDDE` | NotSigned |
| `target/installer/QuetzalcoatlSetup.exe` | `CC01D918306ED969C6B4D7EADA8056B7C69AD6AEDDC8CCFE15B1CADC2F275A69` | NotSigned |
