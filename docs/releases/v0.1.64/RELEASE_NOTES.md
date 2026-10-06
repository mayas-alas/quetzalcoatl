# GnX Node v0.1.64 — integration candidate

Validation is in progress. No claim of completed personal passkey authentication or real NetBird peer connectivity.

## Changes

- Pocket ID, combined NetBird server (Management/Signal/Relay/STUN), and separate rootless dashboard in the existing managed LXC.
- Digest-pinned offline images; no guest registry pulls, no application underlay egress.
- Caddy private-CA gateway routes, HTTP/2 gRPC, WebSocket relay and mesh-only STUN forwarding.
- Credential-preserving runtime reconciliation after gateway activation; stopped guests are not created/woken by updates.
- Protected fresh-instance recovery owner/PAT, confidential Pocket OIDC client and NetBird provider registration. Existing unowned accounts are not reset/adopted.
- STUN obtains the enrolled gateway address from the sidecar rather than relying on missing host iproute2.
- Protocol-aware read-only ACL preflight prevents attempting a workload update over denied transport.
- JSON runtime errors are parsed independent of serialization whitespace; only bounded stable codes reach the progress store.
- PWA service links/readiness and sanitized diagnostics for all required protocols.

## Deployment context

With explicit operator authorization, a new tailnet grant was saved for gateway `100.67.195.81` → managed LXC `100.127.95.60`, only `tcp:1411`, `tcp:8080`, `tcp:8081`, `udp:3478`. Existing grants were preserved; no public/LAN access was granted. Pocket discovery immediately became reachable through private-CA HTTPS.

Previous candidates are not accepted releases: v0.1.60 failed incomplete update reconciliation; v0.1.61 failed an offline manifest pin; v0.1.62 failed missing host `ip`; v0.1.63 deployed the STUN fix but encountered the external ACL denial. An investigation-only diagnostic refresh caused a subsequent payload hash mismatch; this candidate packages the complete reviewed files and a fresh manifest instead of bypassing integrity checks.

## Sealed build input

- Dataset: `gnx-labs-mx/gnx-node-releases`
- Revision: `a919bfcc55a1e95a0eed949d576b880c33b61dcc`
- Path: `build-inputs/v0.1.60/offline.tar.gz` (shared immutable input; archive content unchanged)
- SHA-256: `3afc69a6806442c0d5d372e62a0397b94f24bbc2b09a64eb64d9270f742bba49`
- The Space no longer tracks the archive; build-time retrieval verifies its exact hash.

## Validation

The candidate compiled and installed (Burn exit 0). Managed reconciliation preserved the running LXC and reached OIDC enrollment, but failed `ACCESS_ENROLL_HTTP_422`. Owner setup had succeeded; public Pocket discovery and NetBird API were reachable via the real gateway. Additional read-only preflight confirmed that return-path LXC → node DNS/HTTPS was denied. A narrow reverse grant for TCP 443, TCP/UDP 53 was then saved in the tailnet with the operator's service-integration authorization. Diagnostic-only files were refreshed after the build; this candidate was not promoted/published. Existing source tests passed 66 Rust tests, five memory-only ACL tests and static contracts; these alone are not live acceptance.
