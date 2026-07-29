# 0.2.12 delivery evidence

## Executable gates

| Gate | Command | Result |
|---|---|---|
| Integrated source/build | `powershell -NoProfile -ExecutionPolicy Bypass -File tools/check.ps1` | passed, exit 0 |
| Repository validators | five `tools/validation/*.py` validators | all `ok` |
| Rust tests | `cargo test --workspace --all-targets --locked` | 61 passed |
| Installer build | `installer/build.ps1` via integrated gate | passed |
| Installed payload | administrative MSI extraction and validation | passed |

## Artifacts

| Artifact | Version | SHA-256 |
|---|---|---|
| `target/installer/Quetzalcoatl.msi` | 0.2.12 | `C790EFCFB435964692D0FCB1D11C279695CD275D3B13CECE0E7B5AD226BC9083` |
| `target/installer/QuetzalcoatlSetup.exe` | 0.2.12 | `77DE894BFAAA4F4ECE6DBCE9FB82E59F85B10782F3B1567F19602E70BEB7F93F` |

## Physical cycle

| Scenario | Result | Observation |
|---|---|---|
| Upgrade to 0.2.12 | passed | Setup returned 0; one visible bundle and one hidden MSI |
| Healthy precondition | passed | controller `gnx-controller-nytqmmgwwi11cntrl`, all components READY, joined and quorate |
| Uninstall | passed | Setup returned 0 in 22 seconds |
| Product root | passed | `C:\Program Files\Quetzalcoatl` absent |
| Process/service | passed | zero service and tray processes |
| Shell/startup | passed | no product PATH entry or startup shortcut |
| Registration/cache | passed | zero ARP entries and zero matching Burn/MSI caches |
| Preserved dependencies | passed | WSL and Podman executables remain installed |
| Preserved state | passed | ProgramData and service profile remain |
| Reinstall | passed | Setup returned 0 and recovered the same READY controller/quorum |
| Repair | passed | Setup returned 0 and reconverged to the same READY controller/quorum |

## Root-cause evidence

Microsoft Sysinternals Handle identified exact root-directory handles in
`wslhost.exe` and `win-sshproxy.exe` created by the dedicated Podman Machine under
an older working directory. Moving only the service/tray CWD and adding post-MSI
cleanup did not release those persistent processes.

0.2.12 stops only the managed `quetzalcoatl` machine under the service identity,
preserves its data, signals the main Rust service through a private local event,
and lets MSI remove the now-unlocked root. The accepted uninstall log contains no
rollback and the strict postcondition reports:

```text
ArpCount=0 RootExists=False ServiceCount=0 TrayCount=0
PathContainsProduct=False StartupCount=0 BundleCache=False MsiCacheMatches=0
```
