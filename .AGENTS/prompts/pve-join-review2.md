You are correcting the existing uncommitted PVE-01 payload diff after a second architect review. Do not spawn subagents. Read C:\Users\mayas\Quetzalcoatl\AGENTS.md, C:\Users\mayas\Quetzalcoatl\.AGENTS\SCOPE.md, C:\Users\mayas\Quetzalcoatl\.AGENTS\DECISIONS.md, C:\Users\mayas\Quetzalcoatl\.AGENTS\tasks\PVE_JOIN.md, and inspect the full current script before editing. Stay within the existing two-file ownership.

Fix these concrete acceptance defects:

1. The `join` case currently calls join_host and then falls through past the case statement into the controller-only `pvecm create` path. Every join mode must terminate explicitly on success. Prove with a deterministic shell test/stub that `join` can never execute controller cluster creation or controller authkey persistence.
2. When /etc/pve/corosync.conf already exists, do a bounded verify of the pinned expected topology/quorum before returning ready or the stable PVE_JOIN_INCOMPLETE_TOPOLOGY error. Never call pvecm add again in that state, but do not fail instantly on a transient configuration propagation window.
3. Replace purely textual static checks where practical with behavior-oriented shell checks. Git for Windows provides C:\Program Files\Git\usr\bin\sh.exe; at minimum run `sh -n` with it. Keep a narrow static-check mode only where it is meaningful for the installed payload.
4. Reconfirm the secret lifecycle: password absent from argv/log output/persistent files, temporary /run file mode 0600, cleanup on success and all errors, fingerprint not logged. Do not redesign the stdin contract already verified against Proxmox source.
5. Recompute the manifest SHA only after the final script is stable.

Run the payload static check, Git sh syntax check, manifest hash verification, cargo fmt --all -- --check, cargo test --workspace, and git diff --check. Do not commit or push. Return a concise report including the exact behavioral fallthrough test.
