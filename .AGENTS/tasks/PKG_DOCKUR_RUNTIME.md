# PKG-02 — clean-Windows runtime closure

State: `IN_PROGRESS`

Owner: `codex-cli-pkg-runtime` under architect review.

Acceptance:

- `gnx-host-prepare.exe` starts on a clean Windows 11 Dockur guest without `VCRUNTIME140.dll` preinstalled.
- PE dependency inspection and a full installer rebuild prove the correction.
- The same frozen setup is installed interactively in noVNC.
- No controller/member, secret, or release-scope expansion.

Reproduced evidence: `.AGENTS/evidence/E-CI-02-vcruntime140-missing.png` and `.AGENTS/evidence/E-CI-02-setup-failed-0xc0000135.png`.
