# GnX Node v0.1.63 — integration candidate

**Not yet accepted as a complete Pocket ID / NetBird integration.** Browser/passkey and managed gateway protocol acceptance are pending. Do not interpret successful compilation, upload or installer exit code as authenticated integration evidence.

## Changes

- Fix STUN's dependency on missing host `ip`: use the enrolled Tailscale sidecar and absolute sysctl/nft paths.
- Add read-only, protocol-aware mesh-policy preflight. Stop with `ACCESS_MESH_POLICY_REQUIRED` before disturbing the running workload when the external tailnet denies required backend ports.
- Skip futile gateway probes when that prerequisite is missing; expose the exact sanitized readiness checks.
- Add safe diagnostics for backend health, resolved DNS, transport status and gateway error categories. Never print environment/config/authentication state.

- Deploy Pocket ID, the combined NetBird server (Management, Signal, Relay and STUN), and a separate rootless dashboard in the existing GnX-owned LXC.
- Digest-pinned images, `Pull=never`, offline dependency/image bundle, non-root service identities and workload mesh firewall.
- Private Caddy routes for dashboard, OIDC/API, HTTP/2 gRPC and WebSocket relay; mesh-only UDP STUN forwarding.
- Preserve CA, service identity, mesh enrollment, datastore keys and application data on updates. Never reset an existing unowned identity provider account.
- Protected first-instance recovery owner/PAT and Pocket ID confidential OIDC client registration. The operator's personal passkey is not fabricated or automatically enrolled.
- PWA service cards and sanitized readiness probes for discovery, dashboard, gRPC, WebSocket and STUN.
- Explicit installed-runtime update deploys access services only into an already running managed LXC, after gateway activation.
- Reproducible sealed build input is downloaded/verified at build time only; no guest registry download path is introduced.
- Correct the combined server digest to its actual offline-import manifest. The new isolated-store test imports all three access image archives and checks the exact Quadlet references; all three passed before this candidate was built.

## Validation record

- v0.1.60 compiled and installed (Burn exit 0); managed `gnx upgrade --confirm` failed `VERIFY_TIMEOUT`. That candidate was not published as a validated release.
- The v0.1.60 failure exposed missing access-workload application in the Retry/update reconciliation path. Deferred workload reconciliation now follows gateway restart.
- v0.1.61 compiled, passed 65 Rust tests and static contracts, and installed (Burn exit 0). Managed upgrade failed `OFFLINE_ACCESS_IMAGE_REQUIRED`.
- Clean offline-image import reproduced the failure: the server Docker archive loads as digest `c75ce9a743ded882477530932ae060cb810af5d359fa047883ea960059cc1d1a`, not the source-store manifest digest. v0.1.61 was not published as an accepted release.
- v0.1.62 static contracts and isolated offline-import digest tests passed; MSI/Burn compiled and installed (Burn exit 0). Managed `gnx upgrade --confirm` reached DNS publication but failed `STUN_POLICY_FAILED`. Host private-CA HTTPS request to Pocket ID returned 502. This candidate is not yet a validated release and its binary has not been published.
- Development WSL combined-server checks: fresh instance, OIDC discovery, setup PAT response and authenticated identity-provider list succeeded. These are not real managed-LXC or browser/passkey acceptance.
- Offline archive relocation was published independently to Space revision `684c6a62cec35673b24a0f83945451b6982f0dd0`; the Space no longer contains `payload/node/workloads/rustfs/offline.tar.gz`. Dataset revision/size/SHA-256 were independently verified. The remaining integration worktree changes are not included in that storage-only commit.

## Sealed input

The workload archive is shared with the v0.1.60 build input; its content has not changed:

- SHA-256: `3afc69a6806442c0d5d372e62a0397b94f24bbc2b09a64eb64d9270f742bba49`
- Dataset revision: `a919bfcc55a1e95a0eed949d576b880c33b61dcc`
- URL: https://huggingface.co/datasets/gnx-labs-mx/gnx-node-releases/resolve/a919bfcc55a1e95a0eed949d576b880c33b61dcc/build-inputs/v0.1.60/offline.tar.gz

## Live blocker identified

Read-only checks on the actual managed runtime confirmed:

- RustFS backend TCP 443: private-CA HTTPS 200.
- Pocket ID and dashboard: healthy inside the managed LXC; Pocket OIDC discovery succeeds on loopback.
- Backend DNS resolves to the enrolled LXC mesh identity.
- Guest nftables permits the intended mesh ports and denies application underlay egress.
- Resolved Tailscale packet-filter rules permit gateway → LXC TCP 443 but deny TCP 1411/8080/8081 and UDP 3478. Connections to the denied TCP ports time out; Caddy returns 502.
- STUN unit's original failure was `ip: command not found`; no missing IPv4 forwarding (it is enabled).

External policy changes have not been made without operator authorization. A complete release/browser login cannot be accepted while that prerequisite remains denied.

## v0.1.63 installation / diagnostic result

- Built and installed successfully (Burn exit 0); no reboot or re-enrollment.
- Managed upgrade applied the STUN fix: `netbird-stun.service` is now `active (exited)`, exit `0/SUCCESS`, IPv4 forwarding `1`.
- Preflight on the actual runtime passed TCP 443 and failed TCP 1411/8080/8081 and UDP 3478, matching the resolved packet-filter rules.
- Upgrade displayed generic `RUNTIME_FAILED` because the existing Rust error extractor did not accept whitespace in Python's JSON serialization. Source now parses JSON and restricts codes to bounded uppercase/digits/underscores; a memory-only Rust test covers the issue and rejects arbitrary/private text. This post-build change is not in the installed v0.1.63 binary.
- Latest source: 66 Rust tests and all static contracts passed; five memory-only Python ACL tests passed. Linker emitted the existing LNK4098 warning.
- The diagnostic script was refreshed for read-only investigation after the build. No release promotion or binary upload was performed.

## Acceptance still required

- Managed-LXC service installation and preserved existing workload/data.
- Private-CA gateway checks for all protocols, including live relay/STUN.
- Pocket ID personal passkey enrollment, authenticated NetBird dashboard and real peer operation.

The services are private tailnet workloads, not public Internet endpoints or a replacement for GnX's existing Tailscale bootstrap transport. Client route/address overlap with a second overlay must be reviewed before peer enrollment.
