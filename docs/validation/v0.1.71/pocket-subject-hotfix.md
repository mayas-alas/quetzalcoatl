# Hotfix local — Pocket subject, no mailbox dependency

## Observed cause

The operator's real Pocket account completed a passkey attempt, but OAuth2 Proxy
returned 500 at the callback. A short-lived standard-logging diagnostic confirmed
`Error redeeming code during OAuth2 callback: email in id_token (...) isn't verified`.
The account had an email, `emailVerified=false`, and the permitted group had a
member. No PIN, cookie, code, token or credential is retained in this report.

The original isolated fixture always emitted `email_verified=true`, hiding this
integration mismatch. A fixture with a signed subject, the allowed group and
`email_verified=false` failed member acceptance with the original config.

## Fix and validation

- `oidc_email_claim = "sub"`: use the immutable signed subject in the proxy's
  internal identity field, not an email address. The PWA receives no identity
  headers or tokens. Keep signature/issuer/audience, PKCE, nonce, CSRF and group
  checks; do not enable unverified-email or nonce bypass flags.
- 16 isolated real-proxy/Valkey checks pass with the unverified-email fixture:
  member access, no-group denial, refresh revocation, CSRF/nonce, secure cookie,
  logout/server-ticket replay and session-store failure closed.
- Static PWA auth boundary checks pass.
- Applied as a **local hotfix**, to the installed Windows public auth template,
  managed WSL payload template and generated protected config. Only app-auth
  restarted; no new identities/keys, no compute or shared guest restart.
- app-auth/app-sessions/gateway active. After startup, anonymous `/`,
  `/status.json` and `/app.js` return 302, not private content. A single immediate
  startup request saw 502; subsequent checks pass.
- Temporary standard logging disabled again; request/auth logging stayed off.

This is not a rebuilt installer or published new release. The original v0.1.71
artifact remains immutable. After the operator's Windows Hello step, Firefox
opened the real authenticated `https://app.gnx/` PWA. Accessibility inspection
confirmed **Nodo operativo / 100%**, the Windows/WSL, Podman, network, HTTPS and
compute checks correct, and RustFS/Pocket/NetBird operational. This demonstrates
real member login and authenticated status rendering after the fix. The session
was left open for the operator; real-session logout/replay and revocation were
not exercised again (covered only by the isolated checks in this hotfix).
The original v0.1.71 `payload_current` evidence predates this hotfix and does not
validate the modified payload.

## Shared guest inventory at inspection

All six managed units active: rustfs, pocket-id, netbird-server,
netbird-dashboard, workload-mesh and workload-mesh-guard. Their five Podman
containers were running, RustFS/Pocket/server/dashboard healthy.
`continuity-business` also running (`localhost/continuity:0.2.0`,
`PODMAN_SYSTEMD_UNIT=continuity.service`); it is an additional application, not
part of the PWA authentication fix. It was not modified.

RustFS credentials confirmed at `/var/lib/gnx/rustfs/rustfs.env` inside LXC 100,
mode 600/root. Only variable names and permissions inspected; no values printed.
