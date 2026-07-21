# CI-03 — evidence-driven completion for interactive Dockur

You are a Codex CLI implementation agent using `medium` reasoning in the `windows-rdp-tailscale` harness worktree. Do not spawn subagents, commit, push, dispatch workflows, publish releases, or touch product code.

## Reproduced acceptance flaw

GitHub run `29862942345` concluded `success` after its fixed session timer even though the end-user install had failed with missing `VCRUNTIME140.dll`, and the guest collector also failed. A green interactive run must not be possible without valid exported guest evidence.

## Objective

Change only the bounded-session/evidence logic so the workflow:

1. polls the existing host-mounted `$RUNNER_TEMP/gnx-shared/gnx-evidence/gnx-evidence.json` during the interactive window;
2. ends the wait early only when that file is complete, parseable JSON and proves the exact expected installer SHA plus at least one detected GNX service and at least one installed binary hash;
3. treats a valid `gnxStatusJson` **or** an explicit `gnxStatusError` as acceptable collection output, because missing interactive cluster secrets may honestly leave the product at configuration-waiting for G2;
4. fails the step when the session expires without valid evidence, so the overall run cannot be green merely because Dockur stayed alive;
5. preserves the 60/120/180 minute hard bound, artifact upload/cleanup `if: always()`, one-day retention, pinned image, tailnet exposure, and the distinction from G5 physical quorum evidence.

Use Python `utf-8-sig` or an equally robust parser so Windows PowerShell's possible UTF-8 BOM is accepted. Never print JSON contents or secret-bearing values. A short redacted reason such as missing field/hash mismatch is acceptable, but do not echo status payloads.

Update the harness README and job summary only where needed to state the evidence-driven success rule.

## Verification

Run actionlint, parse all embedded PowerShell blocks, ShellCheck all embedded Bash blocks, unit-test the validator with temporary valid/invalid JSON fixtures (including BOM), and run `git diff --check`. Report exact files and commands. No commit or push.
