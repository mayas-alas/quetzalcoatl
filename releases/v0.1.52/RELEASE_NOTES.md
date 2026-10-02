# GnX Node v0.1.52

## RustFS access fixed in the installed product

- Dashboard: https://rustfs.gnx/rustfs/console/.
- The existing Caddy gateway uses the existing private CA and verifies native HTTPS to the RustFS Tailscale sidecar. No Serve, Funnel, public ingress, alternate gateway or insecure TLS bypass.
- Private DNS publishes `rustfs.gnx` at the node gateway and `rustfs.mesh.gnx` at the enrolled sidecar. Windows local access includes the fixed RustFS hostname over the existing loopback adapter.
- Caddy preserves the original Host header for RustFS SigV4 authentication; its default HTTPS Host rewrite would otherwise break login.
- Retry reconciles CA, DNS and the workload adapter from the installed payload without re-enrolling the mesh or recreating unchanged compute. A running workload requires gateway HTTPS before being reported healthy.
- Existing data, credentials and sidecar identity are preserved. Workload dependencies and OCI inputs remain sealed/offline.

## Installed-host validation (2026-10-02)

- MSI/Burn build and artifact verification passed; Burn upgrade exited 0.
- Installed version is 0.1.52; `gnx retry` returned NodeReady.
- 62 Rust tests, static installer/runtime/network/PWA/workload contracts and shell syntax checks passed.
- Canonical live verification passed all 27 checks, including `dns_rustfs` and `rustfs_gateway_https`.
- Dashboard returned trusted HTTPS 200 from Windows through Caddy; Firefox displayed the real login screen.
- Authenticated admin account-info and STS session authentication through Caddy passed using existing credentials only in protected process memory. No credential values were published.
- Own DNS: public RustFS entry is the node gateway 100.67.195.81; backend-only entry is sidecar 100.127.95.60.
- Tailnet policy adds only gateway node 100.67.195.81 to tag:gnx-services TCP 443; prior grants are preserved.

Phone login, Actions exit-node/private Internet egress, fail-closed exit disappearance and clean-machine reusable-key onboarding are not claimed by this upgrade validation.

## Downloads and retention

Use `GnxNodeSetup.exe` for normal installation; `GnxNode.msi` is the direct MSI. Verify against `SHA256SUMS.txt`.

Only the latest validated release retains installers and hashes. Previous releases retain Markdown notes only. Future publication uses `installer/scripts/verify.ps1 -PublishRelease`; historical Git/LFS objects are not purged.
