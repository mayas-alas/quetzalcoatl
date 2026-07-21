# PKG-02 — remove the clean-Windows CRT dependency

You are a Codex CLI implementation agent working inside the current Quetzalcoatl checkout. Use `medium` reasoning. Do not spawn subagents. Do not commit, push, publish releases, edit evidence/score documents, or broaden scope.

## Reproduced failure

On a freshly installed Windows 11 guest from the pinned `dockurr/windows` image, the frozen setup EXE reached `Processing: Windows Subsystem for Linux` and then Windows displayed:

`gnx-host-prepare.exe - System Error: The code execution cannot proceed because VCRUNTIME140.dll was not found.`

The Burn bundle then ended with `0xc0000135 - Unspecified error`.

## Objective

Make the installer-produced `gnx-host-prepare.exe` run on a clean supported Windows 11 machine without assuming the Visual C++ redistributable is preinstalled. Prefer the smallest reliable solution, normally static CRT linkage for this narrowly scoped native helper, unless repository evidence proves that is unsafe.

## Scope guard

- Inspect the native helper source/build and installer packaging.
- Change only files required to remove or satisfy this dependency.
- Do not alter clustering, controller/member behavior, Tailscale, Proxmox, secrets, release versions, or GitHub workflows.
- Do not add imagined platforms or use cases.
- Preserve reproducible/locked build behavior.

## Required verification

1. Build the affected helper and the complete installer using the repository scripts.
2. Inspect the resulting helper's PE imports with an available first-party/toolchain utility (`dumpbin`, `llvm-objdump`, or equivalent). Prove that `VCRUNTIME140.dll` and related dynamically required VC runtime DLLs are absent, or document and implement a correctly chained redistributable if static linkage is impossible.
3. Run the relevant syntax/tests plus `git diff --check`.
4. Report exact changed files, commands, hashes of rebuilt MSI/setup, and remaining limitations. Never print secret values.

Stop and report if the minimal fix would require an architectural or security change.
