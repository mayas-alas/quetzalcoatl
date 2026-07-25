#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def fail(message: str) -> None:
    print(f"installer-resume-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    lock = json.loads((ROOT / "installer" / "dependencies.lock.json").read_text(encoding="utf-8"))
    artifacts = {item["id"]: item for item in lock["artifacts"]}
    dependency = (ROOT / "crates" / "gnx-host-preflight" / "src" / "dependency.rs").read_text(encoding="utf-8")
    normalized = dependency.replace("_", "")
    for artifact_id in ("wsl", "podman"):
        artifact = artifacts[artifact_id]
        for value in (artifact["version"], artifact["file_name"], artifact["sha256"], str(artifact["size"])):
            if value not in normalized:
                fail(f"helper constants differ for {artifact_id}: {value}")

    staging = (ROOT / "crates" / "gnx-host-preflight" / "src" / "staging.rs").read_text(encoding="utf-8")
    for marker in (
        'join("Quetzalcoatl").join("Installer")',
        'join("cache")',
        'expected_sha256',
        'expected_size',
        'sync_all()',
        'msi.partial',
    ):
        if marker not in staging:
            fail(f"stable staging marker is absent: {marker}")

    journal = (ROOT / "crates" / "gnx-host-preflight" / "src" / "journal.rs").read_text(encoding="utf-8")
    for marker in ("install-state.json", "MAX_ATTEMPTS: u8 = 3", "record_error", "product_version"):
        if marker not in journal:
            fail(f"installer journal marker is absent: {marker}")

    ns = {"w": "http://wixtoolset.org/schemas/v4/wxs"}
    root = ET.parse(ROOT / "installer" / "bundle.wxs").getroot()
    chain = root.find(".//w:Chain", ns)
    if chain is None:
        fail("Burn chain is absent")
    if chain.findall("w:MsiPackage[@Id='Wsl']", ns) or chain.findall("w:MsiPackage[@Id='Podman']", ns):
        fail("WSL or Podman still executes directly from Burn Package Cache")

    contracts = {
        "InstallWsl": ("install-wsl --format json", "WslMsiPayload", artifacts["wsl"]),
        "InstallPodman": ("install-podman --format json", "PodmanMsiPayload", artifacts["podman"]),
    }
    for package_id, (arguments, payload_id, artifact) in contracts.items():
        packages = chain.findall(f"w:ExePackage[@Id='{package_id}']", ns)
        if len(packages) != 1:
            fail(f"expected one {package_id}")
        package = packages[0]
        if package.attrib.get("InstallArguments") != arguments or package.attrib.get("SourceFile") != "$(var.HostPreflight)":
            fail(f"{package_id} helper invocation differs")
        payloads = package.findall(f"w:Payload[@Id='{payload_id}']", ns)
        if len(payloads) != 1 or payloads[0].attrib.get("Name") != artifact["file_name"]:
            fail(f"{package_id} ancillary payload differs")
        exits = {(node.attrib.get("Value"), node.attrib.get("Behavior")) for node in package.findall("w:ExitCode", ns)}
        for required in (("0", "success"), ("1641", "forceReboot"), ("3010", "forceReboot"), (None, "error")):
            if required not in exits:
                fail(f"{package_id} exit mapping omits {required}")

    build = (ROOT / "installer" / "build.ps1").read_text(encoding="utf-8")
    bundle_module = (ROOT / "installer" / "modules" / "bundle.ps1").read_text(encoding="utf-8")
    if "Test-DependencyStagingContract" not in build:
        fail("installer build does not enforce the dependency staging contract")
    for marker in ("-WslMsiPath $artifacts.wsl", "-PodmanMsiPath $artifacts.podman"):
        if marker not in build:
            fail(f"built-bundle dependency verification is not wired: {marker}")
    for marker in ("wsl.2.7.10.0.x64.msi", "podman-installer-windows-amd64.msi", "embeddedDependencyHash"):
        if marker not in bundle_module:
            fail(f"built-bundle ancillary payload verification is absent: {marker}")
    if not re.search(r"ExpectedPayloadVersion\s+5", build):
        fail("installer build does not require runtime payload version 5")

    print("installer-resume-validation: ok")


if __name__ == "__main__":
    main()
