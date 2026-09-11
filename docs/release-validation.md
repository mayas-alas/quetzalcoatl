# 0.3.2-rc.2 verification — 2026-09-11

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

Public artifacts have SHA-256 identities in `manifest.json`. Obtain its digest
through the authenticated GitHub release channel before installation. The
executables do not claim Authenticode signing.
