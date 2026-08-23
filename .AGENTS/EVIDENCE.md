# 0.2.41 delivery evidence

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
| Authorized QA policy transition | passed | at 2026-08-01 10:21 America/Mexico_City an elevated bounded helper changed only `VerifiedAndReputablePolicyState` from `1` to `0`; after the authorized restart the state remained `0`, `gnx version` returned `gnx 0.2.40`, `gnx status --json` returned READY and the tray process was active |
| Full source gate | passed | six validators, RustSec, format, Clippy and 73 tests passed; one elevated fixture ignored by design |
| Signed installer build | passed for QA, not releasable | official entry point produced coherent 0.2.40 Setup/MSI; wrapper and four Rust binaries report `Valid`, `CN=GNX Labs`, timestamp present |
| 0.2.40 Setup QA SHA-256 | passed | `EE90E972BE6F91F81FC32A4CAFD313DF56EDF68E3D2AD662C01AFC6C1CD7D61E` |
| 0.2.40 MSI QA SHA-256 | passed | `34F686C606D8A228FB076B2A92A9476FC2868D1F4E97FA3D423B6828B5CA1A17` |
| 0.2.39 Setup baseline SHA-256 | passed | `485B7FF861EA66403EC330E94FD30C667E221A172A3BAADA83A9222F1D77957F` |
| 0.2.39 MSI baseline SHA-256 | passed | `803790F63D82E4CB84419E8FA20FFD9A499D15C3403919851340218C0E862248` |
| QA trust source gate | passed | six validators, RustSec, format, warnings-denied Clippy and 74 tests passed on 0.2.41; one elevated reparse-point fixture remains ignored by design |
| QA certificate lifecycle | passed | `create-qa-signing-certificate.ps1` created non-exportable RSA root `23745C9ECE6FB8B477B98756A08533C2AEE72EED` valid through 2036-08-01 and publisher `E42B64C72B59A83F8259D73A394C7DBB3BBFDA2F` valid through 2028-08-01; exported DER files contain no private keys |
| 0.2.41 QA Bundle | passed, not publicly releasable | official `installer/build.ps1 -QaSigning` signed and timestamped all first-party artifacts, MSI, detached Burn engine and Setup; extraction matched five bootstrap copies plus the two exact public QA certificates |
| Production QA-trust exclusion | passed | a `QaTrustEnabled=0` compile/extraction probe contained zero `gnx-qa-*.cer` payloads and exactly the four normal bootstrap executables |
| 0.2.41 Setup QA SHA-256 | passed | `C162E0426B82FFE90EA1B53E0EE552F3D04B6E650DB5167C79D0993DE8C3EBD7` |
| 0.2.41 MSI QA SHA-256 | passed | `63F866642FC83D61968A924BC09490CCBFC142D88CF88FCA2A7E1C5DAA8AC4F6` |

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
| Smart App Control | QA exception applied | state is Off (`0`) after the explicitly authorized restart; CLI 0.2.40 and tray execute again; the product and installer do not change Smart App Control, SmartScreen or Defender |
| Upgrade | passed | QA-chain-signed 0.2.40 to 0.2.41 upgrade exited 0; Setup first imported its pinned public trust and the same controller/platform returned READY |
| Repair | passed | 0.2.41 repair exited 0, reran the idempotent trust package with code 0, kept exactly one root and publisher without private keys, and returned controller/platform to READY |
| Add/Remove Programs | passed | one visible Burn entry and one hidden MSI registration identify Quetzalcoatl 0.2.41 with publisher GNX Labs |
| QA trust distribution | passed | before Setup, neither new certificate existed in LocalMachine; after Setup, the root exists exactly once in `LocalMachine\Root` and the leaf exactly once in `LocalMachine\TrustedPublisher`, both public-only; CLI/tray/service/wrapper signatures are `Valid` under the leaf |
| Forgejo admin show/reset | not exercised | source and packaged contracts pass; this QA signing change restored physical CLI execution but did not disclose or rotate the administrator credential |

<<<<<<< HEAD
## 0.2.42 build and source-gate closure (Agent C)

| Gate | Result | Evidence |
|---|---|---|
| Validator inventory | passed | all six source validators (`repository.py`, `contracts.py`, `remote_execution.py`, `runtime.py`, `platform.py`, `installer.py`) report `ok`; `tools/check.ps1 -SourceOnly` reaches the Rust gate. |
| Validator taxonomy fix | applied | `tools/validation/repository.py` declares `TROUBLESHOOTING.md` (added 2026-08-22) in `EXPECTED_DOCS` and excludes `.kilo` plus `.AGENTS/agentA` from the inventory scan; the .AGENTS live inventory filter now ignores the `agentA/` working area while still rejecting any other unexpected file. |
| Release manifest supersession | applied | `release/manifest.toml` advanced to `version = "0.2.42"`, `release_timestamp_utc = "2026-08-22T21:24:00Z"`; `previous_product_code`/`previous_package_code`/`previous_bundle_id` now point at the actual 0.2.41 package (not stale 0.2.40 GUIDs); `bundle_upgrade_code` re-pinned to `{10B764B2-36AE-4911-A8C8-2F1A2A963769}` after a silent drift to a near-identical but distinct GUID was caught; 0.2.42 product/package/bundle GUIDs rotated to non-colliding values. |
| Contract version advance | applied | `tools/validation/contracts.py` advances the pinned `version` to `0.2.42` and the supersession assertion to `0.2.42 does not identify the superseded 0.2.41 QA package`. |
| QA publisher subject correction | applied | `installer/create-qa-signing-certificate.ps1` and `installer/modules/signing.ps1` no longer emit the malformed `CN=GNX Labs. QA Publisher` (extra period); the closed publisher subject is now exactly `CN=GNX Labs QA Publisher`, matching the build-script gate and the evidence table. |
| 0.2.42 Setup SHA-256 | passed | `FAC7E1AD5A7B625CFC9A17303BD439F72AB5AB7072B151DA7C3367438C4E0697` (unsigned development artifact from `installer/build.ps1 -AllowUnsigned`; not releasable). |
| 0.2.42 MSI SHA-256 | passed | `0EF9124C12DC1ABD55209EBACDF43DE6CE7FE9E03E45156A115D1497F9D02A3F` (unsigned development artifact; not releasable). |
| Closed identity/version audit | passed | MSI reports `ProductVersion=0.2.42`, `ProductCode={F43403AB-A35B-4127-9256-FE79AA4FC00C}`, `UpgradeCode={47D5BD44-D061-407B-913B-47D17EC3BEA9}`, `PackageCode={11794170-00CF-4232-9D0E-9B99AB7706A7}`; Burn `Bundle/@UpgradeCode` is `{10B764B2-36AE-4911-A8C8-2F1A2A963769}`. |
| Release artifact signature | passed, not releasable | `installer-validation` accepts the unsigned 0.2.42 development build; production trust still requires `Test-CodeSigningCertificateTrust -RequireAuthRoot $true` and the pinned `GNX Labs` chain. |

### Pre-existing source-gate blockers (not introduced by the 0.2.42 build lane)

| Blocker | Finding | Lane |
|---|---|---|
| Environment-dependent test | `infrastructure::podman::tests::check_docker_pipe_contention_missing_pipe` fails without a Docker daemon on this host (recorded since the prior 0.2.42 session); blocks `cargo test --workspace`. | Agent C / host env |
| Physical execution | A freshly installed 0.2.42 has not been exercised on this host; the unsigned artifact is built but the elevated Setup run, upgrade from 0.2.41, repair and `gnx status` confirmation are still pending. | Coordinator / Agent C |

## 0.2.42 rename restart fix (Agent B)

| Gate | Result | Evidence |
|---|---|---|
| Diagnostic probe | passed | `podman machine ssh --username root quetzalcoatl` journal (2026-08-22 18:57): `gnx-tailscale-enroll.service: Failed with result 'exit-code'` → `tailscaled.service: Job/start failed with result 'dependency'` → `Dependency failed for tailscaled.service`. Confirms the sticky-failed oneshot dependency blocks the sidecar restart. Both units are inactive/dead after the fresh machine boot. |
| Root cause | confirmed | `runtime/containers/tailscaled.container:3` `Requires=gnx-node-pod.service gnx-tailscale-enroll.service`; `rename`'s `restart_tailscale` never cleared the failed enrollment state unlike `runtime/operations/start-tailscale.sh:3`. |
| Fix | applied | `runtime/commands/gnx-tailscale-rename` `restart_tailscale()` now runs `systemctl reset-failed gnx-tailscale-enroll.service tailscaled.service` before both strategies, refuses to re-enroll without persistent `tailscaled.state`, and dumps `journalctl -r -n 40 -u <unit>` on failure. Success keeps `TAILSCALE_HOSTNAME=updated`. |
| Payload lock | applied | `runtime/payload.lock.json` `commands/gnx-tailscale-rename` version `4 → 5`, SHA-256 `0e207953…9be9d4b → a76fda0470a22b7ba5e53991c0b4e048dd4547d3344248a68e04ad0a51ae8905`; all other entries byte-identical. `runtime_assets.rs` already lists the file unchanged. |
| Shell syntax | passed | `bash -n runtime/commands/gnx-tailscale-rename`; `payload.lock.json` parses as JSON. |
| Change-scoped validators | passed | `runtime.py` ok (recomputes new payload SHA/mode/tree); `remote_execution.py` ok (closed-argv `tailscale-rename` intact). |

### Pre-existing source-gate blockers (not introduced by this change)

| Blocker | Finding | Lane |
|---|---|---|
| Version validator stale | `tools/validation/contracts.py:29` hardcodes release version `0.2.41`, but the tree is intentionally `0.2.42` (`Cargo.toml:12`, `release/manifest.toml:2`) per this delivery; `contracts.py` fails until advanced. | Coordinator / Agent A |
| Documentation taxonomy | `docs/TROUBLESHOOTING.md` (added 2026-08-22) is not in `tools/validation/repository.py` `EXPECTED_DOCS`; `repository.py` fails with `extra=['TROUBLESHOOTING.md']`. | Coordinator / Agent C |
| Environment-dependent test | `check_docker_pipe_contention_missing_pipe` fails without a Docker daemon on this host (recorded in the prior 0.2.42 session); blocks `cargo test`. | Agent C / host env |

`tools/check.ps1 -SourceOnly` therefore does not yet run green; the three failures above are recorded as blockers for the closing lane rather than mutated out of scope. The runtime fix itself is complete and its change-scoped validators pass. Physical `gnx status` verification on a freshly installed 0.2.42 remains pending the Agent C build + install lane.
=======
The installer-driven QA trust objective passes end to end. Public release
acceptance remains separately blocked because Smart App Control enforcement
requires reputation or a publisher represented by Windows AuthRoot; the QA root is
intentionally local and production preprocessing removes it.
>>>>>>> origin/master
