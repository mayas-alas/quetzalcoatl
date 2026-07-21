# CI-02 — Dockur shared evidence path

State: `IN_PROGRESS`

Owner: `codex-cli-ci-evidence` under architect review.

Acceptance:

- Guest evidence chooses an existing writable Dockur share rather than assuming `Z:`.
- Missing share fails clearly and does not claim evidence was exported.
- Host artifact path and one-day retention remain unchanged.
- `actionlint`, embedded script checks, and `git diff --check` pass.

Reproduced evidence: `.AGENTS/evidence/E-CI-02-shared-drive-missing.png`.
