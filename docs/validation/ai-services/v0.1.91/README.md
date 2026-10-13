# AI access recovery · v0.1.91 candidate

In progress; not an accepted or published release.

Source base: `501a7ecf813102e4f3fc792782e23424cc751db4`, with inherited uncommitted
implementation changes. The preceding 0.1.90 candidate is installed, but its
runtime reconcile failed. Preserve both LXC guests, identities, CA and storage.

## Bounded correction

Retry recognizes the durable enrollment checkpoint across failed diagnostic
phases; workload diagnostics retain the lifecycle phase. Fresh blocked nodes
without enrollment still require setup credentials. No runtime state editing.

Both sidecars drop capabilities. A root CLI therefore cannot traverse the prior
daemon-owned 0700 state directory. Preflight confirms default UID status succeeds,
root execution itself succeeds, root LocalAPI status fails permission-denied,
and the socket is 0666 inside the 0700 directory. The installed lifecycle now
sets only the parent directory to root:65532/0770. State keys remain 0600 under
the daemon UID; daemon identity/capabilities remain unchanged.

## Checks

- `cargo test --workspace --offline`: 78 passed, 0 failed. Existing debug linker
  warning LNK4098 remains.
- `node tests/test-static-contracts.mjs` with Git Bash on PATH: passed. The first
  invocation used Windows' WSL bash shim, which cannot parse Windows file paths;
  rerun used the existing native Git Bash toolchain, with no test modification.

## Installed result

- Canonical build and artifact verification passed. Burn installation exit 0;
  installed CLI/agent hashes match `built-artifacts.json`.
- `gnx upgrade --confirm` now completes `NodeReady`, without recapturing setup
  credentials or editing checkpoints.
- `gnx workload ai start --confirm` reaches native certificate issuance but
  fails `MESH_TLS_CERT_REQUIRED`. AI is not accepted.
- Windows HTTPS checks validate the trusted CA and hostname with revocation
  best-effort because this private CA has no CRL. App and business Continuity
  redirect to authentication; AI aliases still return 502. See `windows-https.json`.
- Six compiled MSI cleanup-condition cases pass without performing uninstall.

Applied AI revision, service HTTPS/dashboard, restart, browser and publication
gates remain pending. Build intermediates remain managed by the existing
installer build script; sealed offline archives are reused rather than rebuilt.
