# AI private access · v0.1.89 candidate

Scope: the existing AI LXC and gateway, local Windows access and `.gnx` over
Tailscale. No new control plane, images, machine identity or public exposure.
This directory is acceptance evidence, not a published release.

## Before installation

- Installed version: v0.1.88. Both owned guests exist and are running.
- The reusable enrollment key exists with root ownership, mode 0600, one link.
- AI ingress is closed; its `eth0` has no IPv4 address/default route. The
  sidecar process is active but reports `NoState`; active is not authenticated.
- AI names have no DNS answers and return HTTPS 502 through both loopback and
  the shared guest's Tailscale peer connection.
- The saved NetBird setup API credential returns 401.
- The working stylesheet had lost the base theme.

Sanitized observations: [preflight](preflight/runtime-inspection.jsonl).

## Source corrections

- Restore the base stylesheet and preserve hidden/focus behavior.
- Publish AI aliases at the existing gateway from the installed profile; map
  those same names to loopback on Windows. `ai.gnx` opens the AI section in App.
- Credential recovery resumes the existing LXC lifecycle, restoring guarded
  DHCP before enrollment, TLS, dependencies and Quadlets.
- Keep an authenticated peer's state; wait for reconnection before enrollment.
- Failed mesh observation stays a valid false/attention result; a historical
  key error is not repeated after mesh recovery.
- Recover expired NetBird API access with authorization code/PKCE and the
  original protected local recovery account. Validate the original user ID;
  no password reset, new PAT, database edits or Pocket passkey simulation.
- Keep Pocket at the gateway and use protected file-based header auth on the
  account proxy hop. Strip client-supplied backend credentials. Native LXC TLS
  still validates its `.ts.net` name; `.gnx` retains the existing private CA.

## Pre-build gates

- 76 Rust tests pass (16 protocol, 52 agent, 8 setup); existing debug linker
  warning LNK4098 remains.
- Existing installer/runtime/network/security/auth/UI contracts pass, including
  actual Bash/JQ observer cases: authenticated, unauthenticated, invalid and failed.
- 20 enrollment/recovery tests pass, including origin/state/owner rejection.
- Real local curl verifies Token/Bearer and Pocket API headers.
- Folder/ZIP admission and six pinned Quadlet definitions pass.
- Bash/Python syntax and the Quadlet generator pass; no containers/images were
  installed into the isolated host-user laboratory.

## Installation and acceptance

Build passed once: optimized Rust, MSI and Burn; release builds reported zero
errors/warnings. Artifact hashes: [built-artifacts.json](built-artifacts.json).
Installer exit **0**; installed CLI, agent and payload marker match that build.
All six compiled MSI cleanup-condition cases pass without running uninstall.
The installed `gnx upgrade --confirm` completed with platform NodeReady. AI
transport/DHCP and its original Tailscale identity recovered; the machine-key
fingerprint is unchanged. All four public AI aliases now resolve to the gateway.
The native certificate step is blocked: the CLI was running as the sidecar UID,
which lacks Tailscale LocalAPI certificate/write permission. Service access still
returns 502 and is **not accepted**. Correction is source-only in v0.1.90.

Business Continuity remains active/ready with its original image and storage.
The operator additionally requested its own HTTPS hostname during validation;
that new gateway/auth/access configuration is included in the next candidate.

Service access, personal Pocket login, inference, restart preservation and
publication are not yet accepted. Build/installer success alone is insufficient.
