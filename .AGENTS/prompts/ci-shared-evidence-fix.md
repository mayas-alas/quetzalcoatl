# CI-02 — make Dockur guest evidence reach the host share

You are a Codex CLI implementation agent working inside the `windows-rdp-tailscale` harness worktree. Use `medium` reasoning. Do not spawn subagents. Do not commit, push, dispatch workflows, edit releases, or broaden scope.

## Reproduced failure

The real Dockur Windows 11 guest ran `03-Collect-GNX-Evidence.cmd`, but the helper failed before writing evidence:

`New-Item : Cannot find drive. A drive with the name 'Z' does not exist.`

The same guest exposed an empty Desktop folder at `C:\Users\gnxlab\Desktop\Shared`, created from the container's `/shared` bind mount, while `This PC` showed no `Z:` drive.

## Objective

Make `Collect-GnxEvidence.ps1` select a real writable Dockur host share without assuming `Z:` exists. Prefer `Z:\` when present for compatibility, then the current user's `Desktop\Shared` folder when it exists. Fail closed with a clear message if no supported shared root exists; do not silently write evidence to an unexported local path. Keep the host-side artifact collection contract unchanged (`$RUNNER_TEMP/gnx-shared/gnx-evidence/gnx-evidence.json`).

## Scope guard

- Touch only the Dockur workflow and directly related harness documentation/tests if needed.
- Do not change image digest, release inputs, tailnet exposure, ACL tags, session duration, GNX product code, or physical-cluster claims.
- Do not add secrets or print credentials.
- Do not turn a compatibility lane into quorum evidence.

## Required verification

Run `actionlint`, validate every modified embedded PowerShell/Bash block with the existing harness checks, run `git diff --check`, and report exact files/commands plus any limitation. No commit or push.
