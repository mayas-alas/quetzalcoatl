#!/usr/bin/env python3
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "crates" / "gnx-service" / "src" / "runtime"


def fail(message: str) -> None:
    print(f"cluster-contract-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    topology = (RUNTIME / "topology.rs").read_text(encoding="utf-8")
    if "identity.host_peers.len()" in topology or "more than two existing GNX hosts" in topology:
        fail("topology still has a member-count limit")
    for marker in (
        "prepare_member(status, member)",
        "authorize_member(status, member",
        "persist_member_stage(status, member, \"MEMBER_JOINING\")",
        "verify_member(status, podman, member)",
        "confirm_membership(status, podman, member)",
    ):
        if marker not in topology:
            fail(f"member join coordination marker is absent: {marker}")

    state = (ROOT / "crates" / "gnx-service" / "src" / "state" / "mod.rs").read_text(encoding="utf-8")
    for stage in (
        "MEMBER_PREPARING",
        "MEMBER_AUTHORIZING",
        "MEMBER_JOINING",
        "MEMBER_VERIFYING",
        "MEMBER_CONFIRMING",
    ):
        if stage not in state:
            fail(f"persisted member stage is absent: {stage}")

    operation = (RUNTIME / "remote" / "operation.rs").read_text(encoding="utf-8")
    if "PveClusterConfirmMember" not in operation or '["pve-cluster-create", "confirm-member"]' not in operation:
        fail("typed membership confirmation operation is absent")

    agent = (ROOT / "runtime" / "payload" / "bin" / "gnx-runtime-agent").read_text(encoding="utf-8")
    cluster = (ROOT / "runtime" / "payload" / "bin" / "gnx-pve-cluster-create").read_text(encoding="utf-8")
    for marker in ("confirm-member", "confirm-member-inside", "PVE_MEMBERSHIP=confirmed"):
        if marker not in agent + "\n" + cluster:
            fail(f"runtime membership confirmation marker is absent: {marker}")
    for marker in ("pvecm status", "pvecm nodes", "/cluster/resources", "Quorate:"):
        if marker not in cluster:
            fail(f"membership confirmation evidence is absent: {marker}")

    print("cluster-contract-validation: ok")


if __name__ == "__main__":
    main()
