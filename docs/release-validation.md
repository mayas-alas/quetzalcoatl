# 0.3.2-rc.3 verification — 2026-09-11

Decision: **not accepted as a complete G0-G6 Windows installation**. This is a
functional private-network release candidate; the boundaries below are material.

## Current deployment

- The former operator-owned Ubuntu 24.04 WSL instance used Tailscale identity
  `100.124.251.14`; it is legacy evidence and is no longer the active runtime.
- Windows `GNXRuntime` runs separately as `.\gnx-runtime`, with its own `GNX`
  distribution. Its broker reports `HOST_READY`, but that distribution is not
  enrolled and is not the owner of the working `poc031` deployment.
- The active deployment is therefore **not yet migrated to the required
  dedicated-account boundary**. The operator can administer its Ubuntu distro.
  Working remote HTTPS does not prove BR-W01–BR-W09 acceptance.

## Verified

- Windows: 15 integration tests and one unit test pass. Linux: 16 integration
  tests and one unit test pass. The explicitly destructive old loopback fixture
  remains ignored; it must not replace the active node's private entry address.
- Windows formatting and clippy pass. Both release binaries and the Linux bundle
  are built from this source with locked dependencies and pinned runtime images.
- The former split-DNS check resolved `app.gnx` and `compute.gnx` to
  `100.124.251.14`; that result is historical and must not be used as current
  acceptance evidence.
- All three HTTPS endpoints return 200 through the Windows Tailscale client.
  `tests/verify_client.ps1` reproduces these checks. Windows curl uses
  `--ssl-revoke-best-effort` because the private CA has no revocation endpoint;
  hostname and trust-chain verification remain enabled, with no `-k` override.
- `app.gnx` serves the bundled responsive HTML/CSS/JavaScript welcome page.
  Edge's accessibility tree confirms the loaded page, links and version.
- The intermediate certificate has `CN=GNX Local Auth`. Its root was preserved:
  PEM SHA-256 `3ccfbfb735142e3bd66548ec2402c5810d6e899ddacee6ded509b0de7f92eaaa`.
  Existing roots keep their original subject; new installations use `GNX Root`.
- Runtime doctor, authenticated Compute health and apply report READY. Storage
  is checked through the authenticated Proxmox API, not an anonymous page.

## Recovery repairs

The host's existing startup task only executed `/bin/true`, allowing WSL to stop
when idle. Its local action now keeps a session open, without the former five
minute limit or battery-stop setting. This is a legacy deployment repair, not a
replacement for the dedicated service. GNXRuntime now owns and supervises a
keepalive session for its own fixed GNX distro and closes it on service stop.

Access now requests its dependent DNS and Control units when started again;
their namespace dependencies are retained. Fault injection exposed the missing
dependency: previously Access recovered while DNS and Control remained stopped.
`tests/verify_runtime.py --config PATH --recovery` checks unchanged plan/apply,
container and generated-file stability, every service restart, unexpected Access
exit, revision stability and root preservation. Use only an explicitly selected
test node; this flag deliberately interrupts its services.

The 2026-09-11 run passed idempotence, all four service restarts and unexpected
Access exit. Before/after revision was
`7416b5ec29d82b83f7ce042b48d1e9e10298c71573c8fa6a8903134427b7f4a8`;
the root hash above was unchanged and final status had all capabilities healthy.

## Password behavior

Proxmox login uses `root` with the Linux PAM realm. The existing node's password
is protected Linux runtime state under `compute/password`. GNX supplies it to
Dockurr on every container start, and authenticated health uses that same value.
Changing only the password inside Proxmox is not a persistent GNX password
rotation: it breaks health until synchronized and is overwritten on restart.
No password-export or password-rotation command is claimed in this candidate.

## Outstanding acceptance

- Migration of the working node into the dedicated-account runtime without
  changing identity, root, credentials or storage.
- Complete clean install/uninstall/reinstall and authenticated upgrade/rollback.
  The installer still refuses unrelated existing state; source artifact update
  on this host was a controlled administrative maintenance step.
- Full Windows reboot without operator sign-in, followed by remote checks.
- Android client validation (device was offline), unauthorized-client refusal,
  and the future Windows VM and bare-metal host.
- Complete failure matrix, broker DACL/parity tests and G0-G6 evidence required
  by `poc.md`. A successful build or a running Windows service is insufficient.

## Local lifecycle implementation evidence

The current worktree adds a receipt-gated destructive uninstall path for future
clean instances. It requires the literal `REMOVE-GNX-AND-DATA` confirmation,
verifies the fixed service/account/install roots, and asks `GNXRuntime` to
unregister only its own WSL `GNX` distribution before removing Windows state.
The ownership receipt and request live in the administrator-owned install root,
where the service account has read-only access.

`cargo test --locked --all-targets`, strict Clippy, release binary compilation,
PowerShell parser validation, and invalid-token exit checks pass locally. The
uninstall was deliberately not run against the existing deployment. Clean-host
install/uninstall/reinstall evidence remains outstanding and this section does
not change the acceptance decision above.

The worktree also replaces unsigned-manifest acceptance with a detached Ed25519
signature verified inside `gnx-install.exe` against its pinned release public
key. Unit and CLI tests reject modified manifests, malformed signatures and a
signature from a different key. The private PoC authority seed is protected
outside the repository; only its public key and key identity are versioned.

The signed Windows pipeline completed after the lifecycle changes. The current
manifest SHA-256 is
`117ffb844df76818778ec13e5d0f4dfd3ca65c7f83fdd981ea1ca682ff993f46`,
the signing key identity is
`7720127012478b755554071a15ba57b4874cb99152be37c1fa11620a5f649a63`,
and the complete ZIP SHA-256 is
`6a0455661839100ed88cb301d74e2f9061590b4b3e855413b719229c9f5b4c29`.
The ZIP includes the pinned rootfs. `gnx-install.exe verify`
returned `READY/RELEASE_AUTHENTIC`, while an incorrect manifest digest returned
`FAILED/MANIFEST_AUTHENTICATION_FAILED`. These are build and G0-negative-path
artifacts, not live clean-host installation or Linux execution evidence.

The signed manifest now carries monotonic `release_serial=30203`. The durable
last-valid revision binds both strict operator intent and the embedded immutable
release definition. An executable acceptance test proves that changing the
authenticated release forces reconciliation even when GNX intent is unchanged;
the state store promotes the combined revision atomically with the intent.

The source now contains the corresponding two-phase Windows update and
explicit rollback implementation. It validates signed serial direction, backs
up Windows binaries, stages the Linux bundle through the service-owned WSL
identity, runs Linux `apply` plus `status`, verifies the Windows broker, and
commits the receipt only after both boundaries pass. Pre-commit failures invoke
Linux and Windows rollback. This path has compile, parser and contract-test
evidence only; it has not yet been executed against an isolated installed node.

Public artifacts have SHA-256 identities in `manifest.json`. Obtain its digest
through the authenticated GitHub release channel before installation. The
executables do not claim Authenticode signing.

## Isolated rc.3 preflight and G0 release negatives

The current candidate is **not accepted**. The rc.3 ZIP and signed manifest
above were built locally from the current source; they have not been published
as a GitHub release. The reproducible `tests/verify_release.ps1` invocation
against that ZIP, with the external signing key only for a disposable invalid
schema case, returned `G0_RELEASE_MATRIX_PASSED`: authentic manifest exit 0,
incorrect digest, altered signature, corrupted executable and authenticated
unsupported schema each exit 1 with distinct JSON codes and corrective
`next_action`. Corrupted executable rejection occurs before elevation. This is
only the release-integrity subset of G0; host-prerequisite failures, clean-host
installation and partial-install assertions are still outstanding.

An explicitly isolated WSL2 distribution `GNXAcceptance-rc3` was imported from
the pinned Ubuntu 24.04.5 rootfs into the ignored `.acceptance` directory. It
booted systemd and Podman with automount disabled. The installed GNX Linux ELF
had SHA-256
`d941c33da2ca77b981d366ece62a5feb7485525a527e32463fdfab27ae3451d8`.
After resetting only this lab's uninitialized GNX state, two `plan` calls
returned identical `READY/PLAN_OBSERVED`, exit 0, and created no node-state
directory. An `apply` with a canary `GNX_COMPUTE_PASSWORD` environment variable
returned `ACTION_REQUIRED/COMPUTE_PASSWORD_REQUIRED`, exit 2, and created only
`/var/lib/gnx/gnx/apply.lock`; no candidate, volume directories or last-valid
revision appeared. The lab had images cached from an earlier local attempt, so
these observations do not establish image immutability. The CLI no longer reads
environment secrets and negotiates a typed secret before image/network/volume
mutation. No enrollment key or password was supplied; Compute/Access/Control,
remote clients, Windows clean install/update/rollback and G0-G6 remain unproved.
