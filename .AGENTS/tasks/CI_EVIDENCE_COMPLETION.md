# CI-03 — evidence-driven Dockur completion

State: `IN_PROGRESS`

Owner: `codex-cli-ci-completion` under architect review.

Acceptance:

- A Dockur run cannot conclude success without exported, parseable evidence for the exact installer hash.
- Valid evidence includes GNX service and binary hashes and may honestly report configuration waiting.
- Valid evidence ends the interactive wait early; missing/invalid evidence at the hard deadline fails.
- Cleanup and short-lived redacted artifact upload still run on every outcome.
