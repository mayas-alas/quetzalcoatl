# Windows branding asset provenance

These files are copied from `origin/legacy` and are treated only as historical source material.

| Current file | Legacy source | SHA-256 |
| --- | --- | --- |
| `packaging/windows/assets/branding-install-logo.ico` | `origin/legacy:assets/branding-install-logo.ico` | `1f1e54880c65036a057169a32af3b114d9a7cfe2ab28840f8a389606556ab3a5` |
| `packaging/windows/assets/branding-install-logo.png` | `origin/legacy:assets/branding-install-logo.png` | `ac4b4ea86c58d1a2e61521ff1f31c50a2faec306c2990c6e148ac5a7ff408bad` |
| `packaging/windows/assets/banner-install-side.png` | `origin/legacy:assets/banner-install-side.png` | `d1f12c614f608440d7f43aac2d53c1f54720ed8771216ac221557ce7f0f2f0e0` |
| `packaging/windows/assets/bg-installer-banner.png` | `origin/legacy:assets/bg-installer-banner.png` | `dac8ba6b71216482f4f97ab78cc739125cf067070cda4a647956581ebc03647f` |
| `packaging/windows/assets/tray-icon.ico` | `origin/legacy:assets/tray-icon.ico` | `b0aa0d235a77b722fb54962077725ea20b86176f321ecd98c6940775ec9a1579` |
| `packaging/windows/assets/tray-icon.png` | `origin/legacy:assets/tray-icon.png` | `a259230f9ed86e0b4a1d220530531909460a615e88417f126d8d722858199195` |

## Incorporation boundary

- `packaging/windows/build.ps1` copies these assets into `dist/assets` for Windows package consumers.
- The current setup UI is a minimal PowerShell/NDJSON wrapper, not a commercial installer surface, so these assets are packaged but not rendered by product code yet.
- The current architecture has no supported GNX tray application; the tray icons are packaged as historical branding inputs only and must not be interpreted as evidence of a tray daemon.
