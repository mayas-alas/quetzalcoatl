# CI-02 focused review

Act as an independent Codex CLI reviewer with `medium` reasoning. Do not spawn subagents, edit files, commit, push, or dispatch workflows.

Review only the current uncommitted diff in `windows-rdp-tailscale` for the Dockur guest evidence path correction. The reproduced guest had no `Z:` drive but did have `C:\Users\gnxlab\Desktop\Shared`, backed by the workflow's `/shared` bind mount. The host must still collect `$RUNNER_TEMP/gnx-shared/gnx-evidence/gnx-evidence.json`.

Look specifically for:

- claiming a path is writable without proving it;
- falling back to a guest-local directory that the host cannot collect;
- PowerShell syntax/runtime errors on clean Windows 11;
- secret leakage or broadened network/exposure scope;
- misleading claims that this compatibility lane proves quorum/Corosync;
- encoding or quoting defects in YAML/heredocs.

Run proportionate read-only checks. Return `CLEAN` if there is no actionable issue. Otherwise report only concrete P1/P2 findings with file and line, impact, and the smallest correction.
