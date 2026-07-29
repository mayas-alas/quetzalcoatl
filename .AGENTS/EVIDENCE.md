# 0.2.1 delivery evidence

## Executable gates

| Gate | Command | Status | Evidence |
|---|---|---|---|
| Taxonomy/contracts | five `tools/validation/*.py` validators | passed | repository, contracts, remote execution, runtime and installer all `ok` |
| Format | `cargo fmt --all --check` | passed | integrated gate |
| Lint | `cargo clippy --workspace --all-targets --locked -- -D warnings` | passed | zero warnings |
| Tests | `cargo test --workspace --all-targets --locked` | passed | 52 tests |
| Installer | `installer/build.ps1` | passed | extraction rejects missing lock and accepts complete payload/image |
| Reproducibility | two consecutive installer builds | passed | MSI and Setup hashes identical |
| Integrated | `powershell -NoProfile -ExecutionPolicy Bypass -File tools/check.ps1` | passed | exit code 0 |

## Artifact evidence

| Artifact | Version | SHA-256 | Status |
|---|---|---|---|
| `target/installer/Quetzalcoatl.msi` | 0.2.1 | `8588B39950E00725C706F891EDC7E33EA3A8A8E87474829F25AB54B870F32ED2` | passed |
| `target/installer/QuetzalcoatlSetup.exe` | 0.2.1 | `82B86A21F8DE89002E9E893F174CBCACF02C5F0D435158E47E988633A88C1D04` | passed |

## Physical acceptance

| Scenario | Status | Required observation |
|---|---|---|
| Fresh installation | pending | one Setup completes; service and tray start |
| Upgrade from 0.1.17 | pending | identity/state retained and binaries replaced |
| Recovery from broken 0.2.0 | pending reboot | Setup detected 0.2.0, cached 0.2.1 and registered RunOnce; MSI resumes after Windows restart |
| Repair | pending | prerequisites revalidated and MSI key paths restored |
| Reboot/resume | pending | Burn resumes within bounded journal |
| Service restart | pending | role/controller/member checkpoint retained |
| Tray | pending | G icon; status/version/connect only; validated PVE HTTPS URL |

Source/build evidence must not mark physical scenarios complete.

## Taxonomy adjustment

Canonical branding and WiX/Burn derivatives share the single
`installer/assets` ownership tree. No root `assets` directory remains.

## 0.2.0 incident

- Installed MSI cache: `C:\Windows\Installer\7ecc1f.msi`.
- Cached and rebuilt MSI shared PackageCode
  `{461DD952-DBD0-5692-9A05-FB0D3C8EFF55}` but had different SHA-256 values.
- MSI repair log reported errors 1334/2350 for legacy `runtime/probes` cabinet
  members, returned success and started the service with an empty runtime tree.
- `gnx status` failed closed with `RUNTIME_PAYLOAD_INVALID`.

## Physical recovery progress

`QuetzalcoatlSetup.exe /quiet /norestart` started against the installed broken
0.2.0 fixture. Burn detected the old bundle and MSI as related upgrades, cached all
0.2.1 packages, then stopped at `PrepareWsl` with exit 1641. The new MSI was not
executed. HKLM RunOnce contains the 0.2.1 cached bundle, so completion and runtime
verification require the pending Windows restart.
