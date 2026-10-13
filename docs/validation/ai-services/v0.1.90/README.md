# Continuity business and AI private access · v0.1.90

Candidate, not a published release. Builds on v0.1.89's verified installation,
restored stylesheet, private DNS and preserved Tailscale identity.

## Operator requirement and preservation

Business component: `C:\Users\mayas\continuity` (clean working tree).
Canonical entry: `https://continuity.gnx`; requested spelling `continuty.gnx`
redirects by HTTPS to the canonical host. Existing bucket: `continuity` at
`https://rustfs.gnx`. Scope/prefix remain `business` / `continuity/business`.

Preflight: the existing business service is active and ready, generation 0,
replication enabled, no pending/error. Its image is preserved, not rebuilt or
replaced. [Sanitized preflight](preflight/runtime-inspection.jsonl).

## Bounded source change

- Add a Quadlet **access-only drop-in** to the existing business instance.
  Preserve its image, writer, spool, credentials and remote acknowledgement.
- Permit business HTTP only from the existing gateway's Tailscale IP. No LAN
  listener bypass or direct tailnet-client access to the unauthenticated API.
- Reuse the existing confidential Pocket/Valkey integration, common Caddy auth
  snippet and installed configuration lifecycle. Business auth is configured
  independently of AI TLS/readiness. No new CLI or per-app Rust controller.
- Add local/private DNS, existing-CA TLS names and an App access link.
- Run fixed Tailscale LocalAPI CLI operations as root, as required by upstream
  certificate/write permissions. The underlay daemon remains UID 65532 and its
  identity/state are preserved; no added daemon privileges or identity reset.

The pinned installed Caddy parsed the candidate source with exit 0 without
applying it. The existing business Quadlet plus the access drop-in generated
the intended listener while keeping the original storage/image configuration.

## Acceptance

Build and installation completed; Burn exit 0 and installed CLI/agent SHA256
match `built-artifacts.json` (`installed-artifacts.json`). The installed CLI
reports 0.1.90. `gnx upgrade --confirm` failed with `MESH_STATUS_FAILED`.
Read-only applied inspection preserves both guest owners and the AI machine-key
fingerprint, but AI native TLS is absent and service gateways return 502.
Business replication reports `REMOTE_UNAVAILABLE` with pending work after the
incomplete reconcile; generation remains 0. See `applied/runtime-inspection.jsonl`.

Calling `gnx workloads logs ai` then stranded the blocked checkpoint in
`workloads-diagnose`; subsequent Retry/upgrade incorrectly requested the initial
Proxmox credential. This is a source recovery defect, not proof that an enrolled
node needs reprovisioning. A versioned correction is being validated in 0.1.91.
Windows curl additionally reports `CRYPT_E_NO_REVOCATION_CHECK` for the existing
private CA (`windows-https.json`); no insecure TLS bypass was used.

HTTPS/auth paths, native AI TLS/service readback and successful preservation
after restart remain pending. A process, login page or DNS answer is not acceptance.
No production journal writes, bucket changes, personal passkey simulation or
publication are authorized by this evidence record.

## Containerfile/build authority

- Continuity's canonical recipe is `C:\Users\mayas\continuity\deploy\Containerfile`
  (SHA256 `fb062d2d4907488f0ede2a5c6101fe0e922c090cd791e3491d9be182bb2e9fe4`).
  Its source build script invokes that recipe explicitly with no image/network
  pull. This delivery preserves the existing business image rather than rebuilding it.
- Voice's canonical `C:\Users\mayas\gnx-voice-agent\Containerfile` matches the
  packaged snapshot: SHA256
  `500447b1f05d5e755fdde30e18d051e42add45e5b6df28848dd77b7e2e69bc46`.
  The component working tree is dirty and was not overwritten/adopted.
- RustFS's existing recipe is
  `payload/node/workloads/private-services/Containerfile`.
- The installer builds Rust/MSI/Burn and verifies/packages sealed OCI inputs;
  it does **not** prove that it rebuilt every image. Voice's manifest contains
  recipe/snapshot hashes; full recipe provenance for other inherited image
  inputs is not yet recorded there. Do not claim universal image reproduction.
