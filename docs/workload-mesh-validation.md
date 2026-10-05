# RustFS private dashboard — installed v0.1.52 (2026-10-02)

## Current installed acceptance

v0.1.52 MSI/Burn built and verified; Burn upgrade exited 0. Installed Retry
reconciles CA, DNS and the workload adapter from the external payload without
re-enrolling the mesh or recreating unchanged compute. All 27 canonical live
checks pass, including own DNS and gateway RustFS HTTPS. Windows HTTPS returns
200 through existing Caddy; Firefox displays the real console login screen.
Authenticated admin account-info and STS session authentication through Caddy
also pass, with existing credentials held only in protected process memory.

`rustfs.gnx` now resolves to the existing gateway **100.67.195.81**;
`rustfs.mesh.gnx` is the backend-only sidecar record **100.127.95.60**.
Windows local access maps the public hostname through the existing loopback
adapter. Caddy validates the native backend certificate using our CA and keeps
the original Host header for SigV4 (Caddy 2.11 otherwise rewrites it).
A scoped grant permits source node 100.67.195.81 to tag:gnx-services TCP 443.
No key rotation, Serve/Funnel, public underlay ingress or new gateway was needed.
Running workload health now requires the actual Caddy HTTPS path, not only its
in-guest healthcheck. 62 Rust tests and all static contracts pass.

Phone login and exit-node Internet acceptance remain separate. Download and
retention details are in `docs/releases/v0.1.52/RELEASE_NOTES.md`.

## Historical pre-gateway checkpoint

The following records describe the earlier direct-sidecar validation; they do
not override the installed v0.1.52 DNS and gateway contract above.

### Verified live

- RustFS is **running / healthy**. A browser-shaped request to
  `https://rustfs.gnx/` redirects natively to `/rustfs/console/` and returns 200.
  HTTPS was verified against the existing GnX private CA, without `-k`.
- The leaf has SAN `rustfs.gnx`. Its native filenames are `rustfs_cert.pem` and
  `rustfs_key.pem`; API TLS stays on loopback:9000, console TLS uses port 443.
  No Tailscale Serve, ts.net certificate, Funnel or new gateway was added.
- Own DNS resolves `rustfs.gnx` to the sidecar **100.127.95.60**. The existing
  `.gnx` Split DNS uses Nitro **100.67.195.81**. DNS publication waits for the
  actual record; its public configuration is mode 0644 for the Pi-hole reader.
- Podman, jq, curl, OpenSSL, nftables and both image inputs were installed
  offline before releasing the guest gate. TUN is a native PVE `dev0` device.
- Protected one-shot enrollment succeeded. Auth values never appeared in
  argv/chat/logs. Guest/host enrollment copies and the Windows staged key were
  removed; the clipboard was cleared. The sidecar identity survived a scoped
  guest shutdown/start without re-enrollment.
- Authenticated S3 PUT/GET used the existing protected credentials in memory,
  verified TLS and survived that restart. The acceptance object/bucket were
  subsequently deleted. Existing data and credentials were not replaced.
- Direct guest TCP egress to `1.1.1.1:443` was denied. No exit-node lease is
  needed for peer dashboard access; Internet exit acceptance remains separate.
- `gnx status` uses local read-only GetProgress, not GitHub Actions Status,
  and reports even a failed historical snapshot without treating it as IPC
  unavailability. `gnx actions status` retains its Actions-specific meaning.

Evidence: `build/live-network-system.txt` (copy before subsequent probes),
`build/network-cargo-tests.txt`, `build/network-static-tests.txt`,
`build/network-gate-tests.txt`, `build/network-installer-build.txt` and
`build/network-installer-verify.txt`.

## Sealed supply and native fixes

One offline payload archive contains downloaded Debian packages and immutable
OCI image inputs. Its SHA256 is
`fa21c2bc108c343e2160a653c3d03a7a50233f83a96a61789c89e40031436d1b`.
Installer text normalization excludes binary archives; the payload manifest
hashes the original bytes.

OCI export changes the manifest representation. The archive annotation uses
its actual immutable OCI digest, rather than the upstream multi-platform index
which Podman correctly rejected as a digest mismatch. Image contents were not
changed during annotation canonicalization:

| Input | Upstream index digest | Sealed OCI reference digest |
| --- | --- | --- |
| Tailscale | `9482efa70e5e37180a74ad17e01fb0a43ec4b629cdde1a28a9fb79597bca63e2` | `b6d8956b30e9fd5c6077f24454c5f772f6c4b7754373cd82c0350b5aa6c83f96` |
| RustFS | `ba0a1b53e36f321c0d46f3867104abef169f7bc59c467c664ddac87e7ddc9a8b` | `5732ce075c2788a414bcda63f0f88094dc51389887fb06eadc80bd87775e95a0` |

Runtime pulls are forbidden. Native systemd scopes around PVE create/start
avoid leaving exec processes in the parent cgroup while LXC delegates
controllers. Cold startup awaits the existing mesh service instead of falsely
requiring an immediate Running state.

## Gate and base-node evidence

Kernel `6.18.33.2-microsoft-standard-WSL2` lacks NF_TABLES_BRIDGE but supports
NF_TABLES_NETDEV. The wildcard `veth*` syntax passed nft validation yet did not
filter subsequently-created interfaces; it was rejected by real packet tests.
The exact future `veth<VMID>i0` binding blocked IPv4 and IPv6 before interface
creation, opened both, then cut an established TCP stream on reclose. The
compute Quadlet read-only mounts the sealed host nft executable; PVE libraries
already support it. This does not claim that unsupported PVE bridge/ebtables
features became available.

v0.1.47 MSI/Burn upgrade and `gnx retry` returned 0. Native mesh accept-dns=true
fixed Quad100 SERVFAIL and survived a mesh restart. Base verification passed
25 checks, with the enrolled host state and CA fingerprints unchanged.
Duplicate upstream/legacy NAT was not removed blindly. No global WSL shutdown,
host restart, kernel fork, new runtime service or application control API was
introduced.

## Tailnet and remaining acceptance

The tagged Oppo no longer matched the old user-source grant. A limited grant
was saved for source `100.103.170.35`, destination `tag:gnx-services`, TCP 443
and TCP/UDP 53. Nitro now includes Oppo in its peer view, but the latest live
reading reported Oppo **offline**. Policy/discovery is not a phone-browser test.

v0.1.48 Burn installed with exit 0. Upgrade testing exposed an orphaned native
Netavark loopback:8006 DNAT targeting an address with no live container; direct
backend TLS returned 200 while the stale first rule hid compute.gnx. Only proven
orphan loopback rules were removed, not upstream/guest NAT. All 25 base checks
then passed. Recovery now keeps an enrolled, non-stopped runtime alive at a
blocked checkpoint and starts an unchanged compute Quadlet idempotently instead
of recreating PVE on every Retry. The typed StartWorkload reached NodeReady with
message `RustFS is running in its managed LXC.` and no error code.

v0.1.49 MSI/Burn installed with exit 0; installed `gnx retry` returned 0.
Three consecutive installed `gnx status` calls returned 0. All 25 base checks,
workload running/healthy, browser-root HTTPS 200, own DNS and direct-egress
rejection were reverified after installation. Evidence is also retained in
`build/network-v049-installed-evidence.txt` and `build/network-final-acceptance.txt`.

Remaining: Oppo connected + DNS + trusted HTTPS + actual console login; approved Actions
exit-node selection, guest/container private Internet egress and fail-closed
exit disappearance. Do not infer these from NodeReady or container health.

Tests: 62 Rust tests, static contracts and mocked native gate control-flow
checks pass. Live packet, TLS, S3 persistence and scoped guest restart evidence
is separate from those tests. Hugging Face development commits are published
through the authenticated **host user's GnX-SlimFit WSL**, not the dedicated
runtime identity. No accepted workload release is claimed before remote tests.
