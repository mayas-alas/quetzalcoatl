#!/usr/bin/env python3
from __future__ import annotations

import json
import re
from pathlib import Path

BUNDLE = Path("/usr/share/quetzalcoatl/platform")
SERVICES = {
    "freellmapi": (300, "gnx-freellmapi-1", "freellmapi"),
    "omniroute": (302, "gnx-omniroute-1", "omniroute"),
    "deepseek-dsh": (304, "gnx-deepseek-dsh-1", "harness"),
}


def fail(message: str) -> None:
    raise SystemExit(f"native service policy differs: {message}")


def main() -> None:
    seen_vmids: set[int] = set()
    seen_lxc: set[str] = set()
    seen_tailscale: set[str] = set()
    records: list[str] = []

    for slug, expected_identity in SERVICES.items():
        service_root = BUNDLE / "services" / slug
        policy_path = service_root / "policy.json"
        compose_path = service_root / "compose.yml"
        serve_path = service_root / "serve.json"
        if not all(path.is_file() for path in (policy_path, compose_path, serve_path)):
            fail(f"{slug} assets are incomplete")
        policy = json.loads(policy_path.read_text(encoding="utf-8"))
        expected_keys = {
            "vm_id",
            "lxc_hostname",
            "tailscale_hostname",
            "tag",
            "port",
            "health_path",
        }
        if not isinstance(policy, dict) or set(policy) != expected_keys:
            fail(f"{slug} keys")

        vm_id = policy["vm_id"]
        lxc_hostname = policy["lxc_hostname"]
        tailscale_hostname = policy["tailscale_hostname"]
        tag = policy["tag"]
        port = policy["port"]
        health_path = policy["health_path"]
        if not isinstance(vm_id, int) or vm_id != expected_identity[0]:
            fail(f"{slug} VMID")
        if (
            not isinstance(lxc_hostname, str)
            or lxc_hostname != expected_identity[1]
        ):
            fail(f"{slug} LXC hostname")
        if (
            not isinstance(tailscale_hostname, str)
            or tailscale_hostname != expected_identity[2]
        ):
            fail(f"{slug} Tailscale hostname")
        if not isinstance(tag, str) or not re.fullmatch(
            r"tag:quetzalcoatl-[a-z0-9][a-z0-9-]{0,31}", tag
        ):
            fail(f"{slug} tag")
        if not isinstance(port, int) or not 1 <= port <= 65535:
            fail(f"{slug} port")
        if not isinstance(health_path, str) or not re.fullmatch(
            r"/[a-z0-9._/-]{0,127}", health_path
        ):
            fail(f"{slug} health path")

        seen_vmids.add(vm_id)
        seen_lxc.add(lxc_hostname)
        seen_tailscale.add(tailscale_hostname)
        records.append(
            "|".join(
                (
                    slug,
                    str(vm_id),
                    lxc_hostname,
                    tailscale_hostname,
                    tag,
                    str(port),
                    health_path,
                )
            )
        )

    if seen_vmids != {identity[0] for identity in SERVICES.values()}:
        fail("VMID inventory")
    if seen_lxc != {identity[1] for identity in SERVICES.values()}:
        fail("LXC hostname inventory")
    if seen_tailscale != {identity[2] for identity in SERVICES.values()}:
        fail("Tailscale hostname inventory")
    print("\n".join(records))


if __name__ == "__main__":
    main()
