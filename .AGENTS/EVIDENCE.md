# Delivery evidence

## Executable gates

| Gate | Command | Status | Evidence |
|---|---|---|---|
| Taxonomy/contracts | five `tools/validation/*.py` validators | passed | repository, contracts, remote execution, runtime and installer all `ok` |
| Format | `cargo fmt --all --check` | passed | integrated gate |
| Lint | `cargo clippy --workspace --all-targets --locked -- -D warnings` | passed | zero warnings |
| Tests | `cargo test --workspace --all-targets --locked` | passed | 50 tests |
| Installer | `installer/build.ps1` | passed | WiX build, MSI extraction, binary/image/legal/runtime comparison and Burn inspection |
| Reproducibility | two consecutive `installer/build.ps1` runs | passed | four EXE, MSI and Setup hashes identical |
| Integrated | `powershell -NoProfile -ExecutionPolicy Bypass -File tools/check.ps1` | passed | exit code 0 |

## Artifact evidence

| Artifact | Version | SHA-256 | Status |
|---|---|---|---|
| `target/installer/Quetzalcoatl.msi` | 0.2.0 | `592AC630DFA1D27CAD89A786644A71470D39CEDF068285CCD0C194B7C8C940AB` | passed |
| `target/installer/QuetzalcoatlSetup.exe` | 0.2.0 | `B810FB309CED873EFE56924A7935B3BBC32A4E88AD774E5FEC5255F0895C0944` | passed |

## Physical acceptance

| Scenario | Status | Required observation |
|---|---|---|
| Fresh installation | pending | one Setup completes; service and tray start |
| Upgrade from 0.1.17 | pending | identity/state retained and binaries replaced |
| Repair | pending | prerequisites revalidated and MSI key paths restored |
| Reboot/resume | pending | Burn resumes within bounded journal |
| Service restart | pending | role/controller/member checkpoint retained |
| Tray | pending | G icon; status/version/connect only; validated PVE HTTPS URL |

Source/build evidence must not mark physical scenarios complete.

## Taxonomy adjustment

Canonical branding and WiX/Burn derivatives share the single
`installer/assets` ownership tree. No root `assets` directory remains.
