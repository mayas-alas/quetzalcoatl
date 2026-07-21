# CI-02 review correction

You are a Codex CLI correction agent using `medium` reasoning in the `windows-rdp-tailscale` harness worktree. Do not spawn subagents, commit, push, dispatch workflows, or touch unrelated files.

The independent review found this concrete P2:

`New-Item -Force` does not prove an existing `gnx-evidence` directory is writable. An unwritable `Z:\gnx-evidence` could be selected, preventing fallback to the valid `Desktop\Shared` bind mount, and later `Set-Content` would fail.

Apply the smallest correction: after ensuring each candidate directory exists, write and remove a uniquely named, non-secret probe file inside it. Select `$out` only after both operations succeed. On failure, best-effort remove the probe and continue to the next supported shared root. Do not leave probe files behind. Preserve fail-closed behavior if neither candidate is usable. Normalize any accidental mojibake/curly apostrophe in the modified workflow line to plain ASCII.

Run `actionlint`, parse all embedded PowerShell blocks, validate all embedded Bash blocks with the available ShellCheck path, assert the probe semantics, and run `git diff --check`. Report changed files and commands. No commit or push.
