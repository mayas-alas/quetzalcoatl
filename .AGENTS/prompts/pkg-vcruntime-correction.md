# PKG-02 review correction

You are a Codex CLI correction agent using `medium` reasoning in the main Quetzalcoatl checkout. Do not spawn subagents, touch unrelated files, commit, push, or publish releases.

The independent review found that the new prohibited-DLL regex can miss VC runtime variants such as `MSVCP140_1.dll`. The architect also identified that the `api-ms-win-crt-` alternative must match the full suffix rather than only the literal prefix.

Apply the smallest correction to the PE import gate so it case-insensitively rejects all of these dynamic runtime families:

- `api-ms-win-crt-*`
- `vcruntime<digit>*`
- `msvcp<digit>*`
- `msvcr<digit>*`
- `concrt<digit>*`
- `vcomp<digit>*`
- exact `ucrtbase.dll`

Do not reject ordinary Windows system imports such as `api-ms-win-core-*`, `kernel32.dll`, or `ntdll.dll`. Keep static linkage scoped only to `gnx-host-preflight`.

Re-run the PowerShell syntax check, full installer build (cache reuse is fine), inspect/report the resulting helper imports and artifact hashes, run `cargo fmt --all -- --check`, `cargo test --workspace`, and `git diff --check`. No commit or push.
