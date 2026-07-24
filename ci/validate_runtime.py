#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import re
import stat
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PAYLOAD = ROOT / "runtime" / "payload-v1"
MANIFEST = PAYLOAD / "manifest.json"
RUNTIME_GATE = ROOT / "crates" / "gnx-service" / "src" / "runtime_gate.rs"

EXPECTED_COMPONENTS = {"podman-machine-os", "proxmox", "tailscale"}
EXPECTED_PAYLOAD_PATHS = {
    "bin/gnx-proxmox-entrypoint",
    "bin/gnx-pve-configure",
    "bin/gnx-pve-cluster-create",
    "bin/gnx-tailscale-prepare",
    "bin/gnx-tailscale-rename",
    "bin/gnx-tailscale-enroll",
    "config/node/serve.json",
    "quadlet/gnx-node.pod",
    "quadlet/proxmox.container",
    "quadlet/tailscaled.container",
    "systemd/gnx-tailscale-enroll.service",
}
EXPECTED_MANIFEST_KEYS = {
    "schema_version",
    "payload_version",
    "target",
    "policy",
    "components",
    "files",
}


def fail(message: str) -> None:
    print(f"runtime-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def validate_manifest() -> None:
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if set(data) != EXPECTED_MANIFEST_KEYS:
        fail(f"unexpected manifest keys: {sorted(set(data) - EXPECTED_MANIFEST_KEYS)}")
    if data["schema_version"] != 1 or data["payload_version"] != 1:
        fail("unsupported runtime manifest version")

    components = {entry["id"] for entry in data["components"]}
    if components != EXPECTED_COMPONENTS:
        fail(f"unexpected component set: {sorted(components)}")

    entries = {entry["path"]: entry for entry in data["files"]}
    if set(entries) != EXPECTED_PAYLOAD_PATHS:
        fail(
            "manifest payload paths differ: "
            f"missing={sorted(EXPECTED_PAYLOAD_PATHS - set(entries))}, "
            f"extra={sorted(set(entries) - EXPECTED_PAYLOAD_PATHS)}"
        )

    physical = {
        str(path.relative_to(PAYLOAD)).replace("\\", "/")
        for path in PAYLOAD.rglob("*")
        if path.is_file() and path != MANIFEST
    }
    if physical != EXPECTED_PAYLOAD_PATHS:
        fail(
            "physical payload differs: "
            f"missing={sorted(EXPECTED_PAYLOAD_PATHS - physical)}, "
            f"extra={sorted(physical - EXPECTED_PAYLOAD_PATHS)}"
        )

    for relative, entry in entries.items():
        path = PAYLOAD / relative
        contents = path.read_bytes()
        if b"\r" in contents or b"\0" in contents or not contents.endswith(b"\n"):
            fail(f"payload transport contract failed for {relative}")
        digest = hashlib.sha256(contents).hexdigest()
        if digest != entry["sha256"]:
            fail(f"manifest hash mismatch for {relative}: {digest}")
        expected_mode = int(entry["mode"], 8)
        actual_mode = stat.S_IMODE(path.stat().st_mode)
        if actual_mode != expected_mode:
            fail(f"mode mismatch for {relative}: {actual_mode:04o} != {expected_mode:04o}")


def validate_runtime_contract() -> None:
    source = RUNTIME_GATE.read_text(encoding="utf-8")
    match = re.search(
        r"const PAYLOAD_FILES: \[PayloadSpec; (?P<count>\d+)\] = \[(?P<body>.*?)\n\];",
        source,
        re.DOTALL,
    )
    if not match:
        fail("PAYLOAD_FILES declaration not found")
    paths = set(re.findall(r'PayloadSpec::new\(\s*"([^"]+)"', match.group("body")))
    count = int(match.group("count"))
    if count != len(paths) or paths != EXPECTED_PAYLOAD_PATHS:
        fail(f"runtime payload contract differs: count={count}, paths={sorted(paths)}")
    if 'controller.stage = "READY".into();' not in source:
        fail("controller does not persist the final READY checkpoint")
    if "verify_controller_cluster(" not in source:
        fail("persisted controller cluster is not reverified")


def main() -> None:
    validate_manifest()
    validate_runtime_contract()
    print("runtime-validation: ok")


if __name__ == "__main__":
    main()
