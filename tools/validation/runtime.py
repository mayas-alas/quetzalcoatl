#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUNTIME = ROOT / "runtime"


def fail(message: str) -> None:
    print(f"runtime-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def require(source: str, markers: tuple[str, ...], owner: str) -> None:
    for marker in markers:
        if marker not in source:
            fail(f"{owner} omits {marker!r}")


def main() -> None:
    manifest = tomllib.loads((RUNTIME / "manifest.toml").read_text(encoding="utf-8"))
    lock = json.loads((RUNTIME / "payload.lock.json").read_text(encoding="utf-8"))
    if (
        manifest.get("schema_version") != 1
        or manifest.get("generation") != "proxmox-cluster-v2"
        or manifest.get("payload_contract") != 5
        or lock.get("schema_version") != 1
        or lock.get("payload_version") != 5
    ):
        fail("runtime manifest and payload lock contract differ")
    if manifest.get("layout") != {
        "commands": "commands",
        "operations": "operations",
        "containers": "containers",
        "services": "services",
        "configuration": "configuration",
    }:
        fail(f"runtime semantic layout differs: {manifest.get('layout')!r}")

    seen: set[str] = set()
    for entry in lock["files"]:
        relative = entry["path"]
        if relative in seen or "\\" in relative or ".." in Path(relative).parts:
            fail(f"runtime lock path is invalid: {relative}")
        seen.add(relative)
        path = RUNTIME / relative
        if not path.is_file():
            fail(f"locked runtime file is absent: {relative}")
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != entry["sha256"]:
            fail(f"locked runtime hash differs: {relative}")
        expected_mode = "0755" if relative.startswith("commands/") else "0644"
        if entry["mode"] != expected_mode:
            fail(f"locked runtime mode differs: {relative}")

    installed_roots = ("commands", "configuration", "containers", "services")
    actual_installed = {
        path.relative_to(RUNTIME).as_posix()
        for root in installed_roots
        for path in (RUNTIME / root).rglob("*")
        if path.is_file()
    }
    if actual_installed != seen:
        fail(
            "installed runtime tree differs from the payload lock: "
            f"missing={sorted(seen - actual_installed)!r} "
            f"unlocked={sorted(actual_installed - seen)!r}"
        )
    if any(path.startswith("operations/") for path in seen):
        fail("repository-owned runtime operation is listed as installed payload")
    operations = RUNTIME / "operations"
    if not operations.is_dir() or not any(operations.rglob("*")):
        fail("runtime operations taxonomy is absent")

    reconciler = (
        ROOT / "apps" / "gnx-service" / "src" / "application" / "reconciler.rs"
    ).read_text(encoding="utf-8")
    require(
        reconciler,
        (
            'set_stage(status, "PROXMOX_READY")',
            'set_stage(status, "TAILSCALE_SERVE_APPLYING")',
            "apply_tailscale_serve",
            "wait_for_tailscale_serve",
            "join_member_cluster",
        ),
        "runtime lifecycle",
    )
    if reconciler.index('"PROXMOX_READY"') > reconciler.index('"TAILSCALE_SERVE_APPLYING"'):
        fail("Tailscale Serve is applied before PVE readiness")

    runtime_tests = (
        ROOT / "apps" / "gnx-service" / "src" / "application" / "runtime_tests.rs"
    ).read_text(encoding="utf-8")
    require(
        runtime_tests,
        (
            "topology_matrix_selects_exactly_one_controller_without_a_member_count_limit",
            "builds_and_accepts_only_the_fixed_pve_serve_route",
        ),
        "runtime regression tests",
    )

    topology_source = (
        ROOT / "apps" / "gnx-service" / "src" / "domain" / "topology.rs"
    ).read_text(encoding="utf-8")
    require(
        topology_source,
        (
            'peer.online && peer.hostname.starts_with("gnx-controller-")',
            "Some(controller) => Ok(TopologyDecision::Member",
            "None => Ok(TopologyDecision::Controller)",
        ),
        "topology selection",
    )

    print("runtime-validation: ok")


if __name__ == "__main__":
    main()
