#!/usr/bin/env python3
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PREFLIGHT = ROOT / "crates" / "gnx-host-preflight" / "src"
RUNTIME = ROOT / "crates" / "gnx-service" / "src" / "runtime"


def fail(message: str) -> None:
    print(f"host-profile-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    preflight = (PREFLIGHT / "host_profile.rs").read_text(encoding="utf-8")
    for marker in (
        "host-profile.json",
        "Win32_ComputerSystem",
        "Win32_LogicalDisk",
        "total_memory_mib",
        "machine_memory_mib",
        "machine_cpus",
        "machine_disk_gib",
        '"lab"',
        '"cluster-member"',
        "six_gib_host_gets_a_bounded_lab_profile",
    ):
        if marker not in preflight:
            fail(f"preflight resource marker is absent: {marker}")

    profile = (RUNTIME / "profile.rs").read_text(encoding="utf-8")
    for marker in (
        "HOST_PROFILE_MISSING",
        "HOST_PROFILE_INVALID",
        "HOST_RESOURCES_INSUFFICIENT",
        "managed_wsl_config",
        'memory={}MB',
        "rerun QuetzalcoatlSetup.exe",
    ):
        if marker not in profile:
            fail(f"runtime host-profile marker is absent: {marker}")

    runtime_mod = (RUNTIME / "mod.rs").read_text(encoding="utf-8")
    forbidden = (
        "MACHINE_CPUS",
        "MACHINE_MEMORY_MIB",
        "MACHINE_DISK_GIB",
        "const WSL_CONFIG",
        "memory=8GB",
        "processors=6",
    )
    for marker in forbidden:
        if marker in runtime_mod:
            fail(f"hard-coded runtime resource marker remains: {marker}")

    machine = (RUNTIME / "machine.rs").read_text(encoding="utf-8")
    for marker in (
        "profile.machine_cpus",
        "profile.machine_memory_mib",
        "profile.machine_disk_gib",
        'OsString::from("--cpus")',
        'OsString::from("--memory")',
        'OsString::from("--disk-size")',
    ):
        if marker not in machine:
            fail(f"Podman profile application marker is absent: {marker}")

    reconciler = (RUNTIME / "reconciler.rs").read_text(encoding="utf-8")
    for marker in (
        'set_stage(status, "HOST_PROFILE_LOADING")',
        "load_host_profile()?",
        "configure_wsl(&service_profile, &host_profile)?",
        "ensure_machine(&podman, &image, &host_profile.selected)?",
    ):
        if marker not in reconciler:
            fail(f"runtime profile integration marker is absent: {marker}")

    print("host-profile-validation: ok")


if __name__ == "__main__":
    main()
