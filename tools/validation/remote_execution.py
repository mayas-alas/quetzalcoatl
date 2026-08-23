#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REMOTE = ROOT / "apps" / "gnx-service" / "src" / "infrastructure" / "remote"
EXPECTED = {
    "Ping": ("ping",),
    "PveClusterPrepare": ("pve-cluster-create", "prepare"),
    "PveClusterVerifyNode": ("pve-cluster-create", "verify-node"),
    "PveClusterCreate": ("pve-cluster-create", "create"),
    "PveClusterVerify": ("pve-cluster-create", "verify"),
    "PveClusterJoin": ("pve-cluster-create", "join"),
    "PveClusterConfirmMember": ("pve-cluster-create", "confirm-member"),
    "PveConfigure": ("pve-configure",),
    "TailscalePrepare": ("tailscale-prepare",),
    "TailscaleRename": ("tailscale-rename",),
    "PlatformReconcile": ("platform-reconcile",),
    "PlatformDeploy": ("platform-deploy",),
    "ForgejoAdminShow": ("forgejo-admin", "show"),
    "ForgejoAdminReset": ("forgejo-admin", "reset"),
}


def fail(message: str) -> None:
    print(f"remote-execution-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    operation = (REMOTE / "operation.rs").read_text(encoding="utf-8")
    for variant, argv in EXPECTED.items():
        literal = "&[" + ", ".join(f'"{part}"' for part in argv) + "]"
        if f"Self::{variant}" not in operation or literal not in operation:
            fail(f"closed argv mapping differs for {variant}")

    transport = (REMOTE / "transport.rs").read_text(encoding="utf-8")
    limits = (REMOTE / "limits.rs").read_text(encoding="utf-8")
    for marker in (
        "MAX_REMOTE_INPUT_BYTES",
        "MAX_REMOTE_OUTPUT_BYTES",
        "REMOTE_COMMAND_TIMEOUT",
        "child.try_wait()",
        "child.kill()",
        "Zeroizing::new(input.to_vec())",
    ):
        if marker not in transport and marker not in limits:
            fail(f"bounded transport guard is absent: {marker}")

    rust = "\n".join(
        path.read_text(encoding="utf-8")
        for path in (ROOT / "apps").rglob("*.rs")
    )
    for pattern in (
        r'"(?:ba)?sh"\s*,\s*"-c"',
        r"\.args\(\s*\[\s*\"(?:ba)?sh\"\s*,\s*\"-c\"",
        r"Command::new\([^)]*\)\s*\.arg\(\s*\"-c\"",
    ):
        if re.search(pattern, rust):
            fail(f"forbidden shell-string execution matches {pattern!r}")
    for forbidden in ("caller_provided_argv", "arbitrary_remote_command"):
        if forbidden in rust:
            fail(f"arbitrary remote execution marker remains: {forbidden}")

    payload = (ROOT / "runtime" / "commands" / "gnx-runtime-agent").read_text(
        encoding="utf-8"
    )
    for marker in ("set -eu", "umask 077", "fail_usage", '[ "$#" -eq 1 ] || fail_usage'):
        if marker not in payload:
            fail(f"runtime agent hardening marker is absent: {marker}")
    for forbidden in ("nc -l", "ncat -l", "socat", 'exec "$@"', "0.0.0.0"):
        if forbidden in payload:
            fail(f"runtime agent exposes an unauthorized surface: {forbidden}")

    print("remote-execution-validation: ok")


if __name__ == "__main__":
    main()
