# 0.2.40 delivery evidence

Evidence is recorded only after execution. Historical release notes belong in
`CHANGELOG.md`.

## Source and artifact

| Gate | Result | Evidence |
|---|---|---|
| Repository taxonomy | passed | no suspicious version/old/legacy/copy filenames, Ansible directories or TODO/FIXME/HACK markers; the platform validator rejects empty and unlocked directories |
| Platform staging | passed | source and staging contain 23 locked files plus `platform.lock.json`; installer copies exclusively from that lock |
| Forgejo admin taxonomy | passed | canonical commands are `gnx forgejo admin show` and `gnx forgejo admin reset --confirm`; alternatives are rejected |
| Forgejo admin transport | passed at source | elevated pipe authorization, bounded closed argv, transient stdin, API rotation, verification, zeroization and atomic commit are enforced |
| Duplicate source | passed | only the two required OpenTofu root lock files are byte-identical |
| POSIX syntax | passed | Git Bash `bash -n` accepts `reconcile` and `lxc-service` |
| Cargo dependency tree | passed | duplicate versions are transitive `syn` and `windows-sys` compatibility edges |
| Forgejo JWT contract | passed at source | 32 random bytes use raw URL-safe base64; the known 0.2.38 padded form is rotated |
| Nested MTU contract | passed at source | each sidecar is fail-closed at MTU 1100 after Compose startup |
| Service-wrapper signing | passed at source | release staging signs WinSW and MSI coherence compares its exact hash |
| Closed signature/version inventory | passed at source | build names all five first-party executables; MSI extraction checks service, wrapper, CLI and tray; Burn extraction checks four bootstrap copies, MSI, WiX, WSL and Podman; production requires RSA and Windows AuthRoot |
| Existing 0.2.40 QA extraction | passed, not releasable | the new gate accepted exact 0.2.40 versions, signer `2DB2BE6CBC78F1CCF5159C8B9359DBD46A9D38AF` and timestamps throughout the existing Setup; production trust correctly rejected the self-signed one-element chain |
| Smart App Control chronology | confirmed | Code Integrity first rejected the 0.2.27 bootstrap at 2026-07-30 14:41; `target/qa-smart-app-control-state.json` records the 14:46 transition from Enforce (`1`) to Off (`0`) for self-signed QA; successful 0.2.33-0.2.38 signed builds occurred afterward; later Enforce again rejects 0.2.39/0.2.40 |
| Authorized QA policy transition | pending restart | at 2026-08-01 10:21 America/Mexico_City an elevated bounded helper changed only `VerifiedAndReputablePolicyState` from `1` to `0`; immediate CLI/tray probes remained blocked by the already-loaded policy, proving a Windows restart is required |
| Full source gate | passed | six validators, RustSec, format, Clippy and 73 tests passed; one elevated fixture ignored by design |
| Signed installer build | passed for QA, not releasable | official entry point produced coherent 0.2.40 Setup/MSI; wrapper and four Rust binaries report `Valid`, `CN=GNX Labs`, timestamp present |
| 0.2.40 Setup QA SHA-256 | passed | `EE90E972BE6F91F81FC32A4CAFD313DF56EDF68E3D2AD662C01AFC6C1CD7D61E` |
| 0.2.40 MSI QA SHA-256 | passed | `34F686C606D8A228FB076B2A92A9476FC2868D1F4E97FA3D423B6828B5CA1A17` |
| 0.2.39 Setup baseline SHA-256 | passed | `485B7FF861EA66403EC330E94FD30C667E221A172A3BAADA83A9222F1D77957F` |
| 0.2.39 MSI baseline SHA-256 | passed | `803790F63D82E4CB84419E8FA20FFD9A499D15C3403919851340218C0E862248` |

## Physical execution

| Scenario | Result | Evidence |
|---|---|---|
| Core controller | passed | `overall: ready`, controller, joined and quorate |
| LXC 200 Garage | passed | Docker active/enabled; Garage and sidecar healthy; `tailscale0` MTU 1100 |
| LXC 201 Forgejo | passed | Docker active/enabled; Forgejo and sidecar healthy; `tailscale0` MTU 1100 |
| LXC 202 runner | passed | Docker active/enabled; runner and sidecar running; `tailscale0` MTU 1100 |
| Tailscale enrollment | passed | Garage, Forgejo and runner peers are present; service peers are online and direct |
| Tailscale certificates | passed | both sidecars issued their private tailnet certificate without exposing the key |
| Garage HTTPS | passed | anonymous S3 root returns expected HTTP 403; TLS and request complete in under 0.2 seconds |
| Forgejo HTTPS | passed | root returns HTTP 200; Chrome rendered `GNX Forgejo`, navigation and login against Forgejo 16.0.0 |
| Smart App Control | restored, distribution gate blocked | state is Enforce (`1`); Defender antivirus, real-time and tamper protection remain enabled; Windows blocks the self-signed QA `gnx.exe` |
| Upgrade | passed | signed 0.2.38 to 0.2.39 upgrade completed; controller and platform returned READY |
| Repair | passed | signed 0.2.39 repair exited 0; controller and platform returned READY with the same identity and URL |
| Add/Remove Programs | passed | one visible Burn entry for Quetzalcoatl 0.2.39; the internal MSI is hidden as `SystemComponent=1` |
| Forgejo admin show/reset | blocked | source and packaged contracts pass; physical execution requires installing 0.2.40, which Smart App Control rejects while self-signed |

Source and QA artifact gates pass. Protected-host upgrade and Forgejo-admin physical
acceptance remain blocked because Smart App Control requires reputation or a
certificate chaining to a CA in the Microsoft Trusted Root Program; the local
self-signed QA identity is intentionally insufficient.
