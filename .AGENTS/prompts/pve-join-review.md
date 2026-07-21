You are revising the current uncommitted PVE-01 payload diff after architect review. Do not spawn subagents. Read C:\Users\mayas\Quetzalcoatl\AGENTS.md, C:\Users\mayas\Quetzalcoatl\.AGENTS\SCOPE.md, C:\Users\mayas\Quetzalcoatl\.AGENTS\DECISIONS.md, and C:\Users\mayas\Quetzalcoatl\.AGENTS\tasks\PVE_JOIN.md before editing. Preserve file ownership.

Correct these concrete issues:

1. The official Proxmox source shows `pvecm add` reads a non-TTY password from stdin, so retain stdin password delivery. However, without `--fingerprint` it enables manual certificate verification and may consume more interactive input. Obtain the controller PVE TLS SHA-256 fingerprint over the already verified direct, pinned Tailscale IP using `openssl`, validate its exact format, and pass it via `--fingerprint`; never log it or the password.
2. Add every actually used tool to preflight (`jq` and `openssl` included).
3. Post-join verification must be bounded/retried and require cluster name `quetzalcoatl`, `Quorate: Yes`, the pinned controller hostname/IP ring0 pair, and the member hostname/IP ring0 pair.
4. If a Corosync config already exists but does not verify against that pinned topology, fail with a stable resumable incomplete/conflict error instead of running `pvecm add` again.
5. Do not overstate the UDP `nc` probe: document it as a negative preflight only; functional UDP proof is the successful Corosync join/quorum postcondition and ultimately the physical lab gate.
6. Ensure every new failure path retains secret cleanup and that the payload manifest hash matches.

Add deterministic static checks for these contracts, run all available validation plus workspace tests, and do not commit or push. Return the standard completion report.
