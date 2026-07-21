# PKG-02 focused review

Act as an independent Codex CLI reviewer with `medium` reasoning. Do not spawn subagents, edit files, commit, push, build a release on GitHub, or broaden scope.

Review only the current `installer/build.ps1` diff that responds to this reproduced clean-Windows failure: `gnx-host-prepare.exe` could not start because `VCRUNTIME140.dll` was absent, and Burn failed with `0xc0000135`.

The implementation builds `gnx-host-preflight` with `-C target-feature=+crt-static`, then uses a PowerShell PE import parser to fail the build if dynamic VC/UCRT DLLs remain. The helper is later packaged/renamed as `gnx-host-prepare.exe`.

Check especially:

- whether a later Cargo command can overwrite the static helper;
- whether the PE32+ import-directory offsets/RVA mapping are correct and bounded enough for this build gate;
- whether the prohibited-DLL regex can miss common VC/UCRT imports;
- whether the packaged file is exactly the inspected file;
- reproducibility or cache/stale-output hazards;
- accidental changes to unrelated binaries or runtime behavior.

Use read-only commands and inspect the built artifacts as needed. Return `CLEAN` if no actionable issue exists. Otherwise report only concrete P1/P2 findings with file/line, impact, and smallest correction.
