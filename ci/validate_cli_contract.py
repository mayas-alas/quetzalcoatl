#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
import tomllib
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.1.13"


def fail(message: str) -> None:
    print(f"cli-contract-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    manifest = tomllib.loads(
        (ROOT / "crates" / "gnx-cli" / "Cargo.toml").read_text(encoding="utf-8")
    )
    if manifest["package"]["version"] != VERSION:
        fail("CLI crate version differs")
    binaries = manifest.get("bin", [])
    if (
        len(binaries) != 1
        or binaries[0].get("name") != "gnx"
        or binaries[0].get("path") != "src/main.rs"
    ):
        fail("CLI binary identity differs")

    main_source = (ROOT / "crates" / "gnx-cli" / "src" / "main.rs").read_text(
        encoding="utf-8"
    )
    for command in ("gnx status [--json]", "gnx configure", "gnx restart"):
        if command not in main_source:
            fail(f"CLI usage omits {command}")
    for label in (
        "overall:",
        "stage:",
        "role:",
        "controller:",
        "service:",
        "wsl:",
        "podman_machine:",
        "kvm:",
        "tailscale:",
        "tailscale_serve:",
        "proxmox:",
        "cluster_joined:",
        "cluster_quorate:",
    ):
        if label not in main_source:
            fail(f"human status omits {label}")
    if "human_status_exposes_the_complete_mvp_contract" not in main_source:
        fail("CLI human status regression test is absent")

    pipe_source = (ROOT / "crates" / "gnx-cli" / "src" / "pipe.rs").read_text(
        encoding="utf-8"
    )
    for marker in (
        "decode_status_response",
        "decode_operation_response",
        "PROTOCOL_SCHEMA_VERSION",
    ):
        if marker not in pipe_source:
            fail(f"CLI protocol guard is absent: {marker}")

    protocol = (ROOT / "crates" / "gnx-protocol" / "src" / "lib.rs").read_text(
        encoding="utf-8"
    )
    command_body = re.search(r"pub enum Command\s*\{(?P<body>.*?)\}", protocol, re.S)
    if command_body is None:
        fail("protocol command enum is absent")
    commands = {
        item.strip().rstrip(",")
        for item in command_body.group("body").splitlines()
        if item.strip()
    }
    if commands != {"Status", "Configure"}:
        fail(f"Named Pipe command set differs: {sorted(commands)}")

    ns = {"w": "http://wixtoolset.org/schemas/v4/wxs"}
    package_root = ET.parse(ROOT / "installer" / "package.wxs").getroot()
    cli_components = package_root.findall(".//w:Component[@Id='CliComponent']", ns)
    if len(cli_components) != 1:
        fail("MSI must contain exactly one CliComponent")
    files = cli_components[0].findall("w:File[@Id='GnxCli']", ns)
    if (
        len(files) != 1
        or files[0].attrib.get("Name") != "gnx.exe"
        or files[0].attrib.get("KeyPath") != "yes"
    ):
        fail("MSI CLI file contract differs")
    environments = cli_components[0].findall("w:Environment[@Id='SystemPath']", ns)
    if len(environments) != 1:
        fail("MSI CLI PATH registration is absent")
    environment = environments[0].attrib
    expected_environment = {
        "Name": "PATH",
        "Value": "[INSTALLFOLDER]",
        "Action": "set",
        "Part": "last",
        "Permanent": "no",
        "System": "yes",
    }
    for name, value in expected_environment.items():
        if environment.get(name) != value:
            fail(f"MSI CLI PATH attribute differs: {name}")

    build = (ROOT / "installer" / "build.ps1").read_text(encoding="utf-8")
    for marker in ("'gnx-cli'", "target\\release\\gnx.exe", "-CliBinary $gnxCli"):
        if marker not in build:
            fail(f"installer CLI build/coherence marker is absent: {marker}")
    msi = (ROOT / "installer" / "modules" / "msi.ps1").read_text(
        encoding="utf-8"
    )
    for marker in (
        "[string] $CliBinary",
        'Filter "gnx.exe"',
        "staged gnx.exe differs",
    ):
        if marker not in msi:
            fail(f"MSI CLI verification marker is absent: {marker}")

    print("cli-contract-validation: ok")


if __name__ == "__main__":
    main()
