# Execution control

This directory is the operational source of truth for closing the Quetzalcoatl PoC/MVP.

- `SCOPE.md`: fixed product boundary and acceptance contract.
- `DECISIONS.md`: architecture and orchestration decisions.
- `TRACKER.md`: weighted completion score and current work state.
- `EVIDENCE.md`: accepted proof and evidence format.
- `tasks/`: bounded assignments for Codex CLI agents.
- `prompts/`: launch prompts that bind an agent to one task.

Only the primary architect updates scores, closes gates, or changes scope. Agents may propose tracker updates in their final report but must not award their own points.

Statuses are `NOT_STARTED`, `IN_PROGRESS`, `BLOCKED`, or `VERIFIED`. `VERIFIED` requires reproducible evidence recorded in `EVIDENCE.md`.
