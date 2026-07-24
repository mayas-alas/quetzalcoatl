#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import os
import re
import stat
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PAYLOAD = ROOT / "runtime" / "payload"
MANIFEST = PAYLOAD / "manifest.json"
RUNTIME_ROOT = ROOT / "crates" / "gnx-service" / "src" / "runtime"
RUNTIME_CONTRACT = RUNTIME_ROOT / "mod.rs"
EXPECTED_COMPONENTS = {"podman-machine-os", "proxmox", "tailscale"}
EXPECTED_PAYLOAD_PATHS = {
    "bin/gnx-runtime-agent",
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


def expected_mode(relative: str) -> int:
    return 0o755 if relative.startswith("bin/") else 0o644


def validate_manifest() -> None:
    if not MANIFEST.is_file():
        fail(f"runtime manifest is absent: {MANIFEST}")
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if set(data) != EXPECTED_MANIFEST_KEYS:
        fail(f"unexpected manifest keys: {sorted(set(data) ^ EXPECTED_MANIFEST_KEYS)}")
    if data["schema_version"] != 1 or data["payload_version"] != 4:
        fail("unsupported runtime manifest version")

    components = {entry["id"] for entry in data["components"]}
    if components != EXPECTED_COMPONENTS:
        fail(f"unexpected component set: {sorted(components)}")

    entries = {entry["path"]: entry for entry in data["files"]}
    if len(entries) != len(data["files"]):
        fail("runtime manifest contains duplicate file paths")
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
        mode = int(entry["mode"], 8)
        if mode != expected_mode(relative):
            fail(f"manifest mode mismatch for {relative}: {mode:04o}")
        if os.name == "posix":
            actual_mode = stat.S_IMODE(path.stat().st_mode)
            if actual_mode != mode:
                fail(f"filesystem mode mismatch for {relative}: {actual_mode:04o} != {mode:04o}")


def validate_runtime_contract() -> None:
    source = RUNTIME_CONTRACT.read_text(encoding="utf-8")
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

    all_runtime_source = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted(RUNTIME_ROOT.rglob("*.rs"))
    )
    payload_parser = (RUNTIME_ROOT / "payload.rs").read_text(encoding="utf-8")
    if "manifest.payload_version != 4" not in payload_parser or "expected_payload_version=4" not in payload_parser:
        fail("Rust payload parser does not enforce payload version 4")
    runtime_tests = (RUNTIME_ROOT / "tests.rs").read_text(encoding="utf-8")
    if "expected_files=12" not in runtime_tests or "manifest_files=11" not in runtime_tests:
        fail("Rust payload contract test does not reflect the 12-file payload")

    required_contract = {
        'const RUNTIME_GENERATION: &str = "proxmox-cluster-v2";': "runtime generation changed unexpectedly",
        "read_runtime_generation(": "managed machine generation is not inspected",
        "remove_managed_machine(": "incompatible managed machines are not recreated",
        "read_managed_tailscale_state(": "network identity is not preserved during recreation",
        "reset_runtime_checkpoint()": "cluster checkpoint is not reset after recreation",
        "write_runtime_generation(": "runtime generation is not committed",
        "verify_runtime_agent(": "Fedora runtime agent is not verified after payload application",
        'controller.stage = "READY".into();': "controller READY checkpoint is not persisted",
        "verify_controller_cluster(": "persisted controller cluster is not reverified",
        "reconciler::run(&status)": "runtime facade does not invoke the reconciler",
    }
    for fragment, message in required_contract.items():
        if fragment not in all_runtime_source:
            fail(message)


def validate_shell_payload() -> None:
    for path in sorted((PAYLOAD / "bin").iterdir()):
        if not path.is_file():
            continue
        result = subprocess.run(["sh", "-n", str(path)], capture_output=True, text=True)
        if result.returncode != 0:
            fail(f"shell syntax failed for {path.name}: {result.stderr.strip()}")


def validate_layout() -> None:
    for legacy in (ROOT / "runtime" / "payload-v1", ROOT / "runtime" / "payload-v2"):
        if legacy.exists():
            fail(f"legacy payload directory remains: {legacy}")
    if (ROOT / "crates" / "gnx-service" / "src" / "runtime_gate.rs").exists():
        fail("legacy runtime_gate.rs monolith remains")


def main() -> None:
    validate_manifest()
    validate_runtime_contract()
    validate_shell_payload()
    validate_layout()
    print("runtime-validation: ok")


if __name__ == "__main__":
    main()
