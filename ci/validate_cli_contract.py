#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
import tomllib
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.1.15"


def fail(message: str) -> None:
    print(f"cli-contract-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    cli_root = ROOT / "crates" / "gnx-cli"
    manifest = tomllib.loads((cli_root / "Cargo.toml").read_text(encoding="utf-8"))
    if manifest["package"]["version"] != VERSION:
        fail("CLI crate version differs")
    binaries = manifest.get("bin", [])
    if len(binaries) != 1 or binaries[0] != {"name": "gnx", "path": "src/main.rs"}:
        fail("CLI binary identity differs")

    commands = (cli_root / "src" / "commands" / "mod.rs").read_text(encoding="utf-8")
    for command in ("gnx status [--json]", "gnx configure", "gnx restart", "gnx -v", "gnx --version"):
        if command not in commands:
            fail(f"CLI usage omits {command}")
    for variant in ("Status", "Configure", "Restart", "Version"):
        if variant not in commands:
            fail(f"CLI action set omits {variant}")


    version = (cli_root / "src" / "commands" / "version.rs").read_text(encoding="utf-8")
    if 'env!("CARGO_PKG_VERSION")' not in version or 'println!("gnx {}"' not in version:
        fail("CLI version output is not bound to the crate version")
    if "Action::Version => version::run()" not in commands:
        fail("CLI version path is not local to the CLI")

    output = (cli_root / "src" / "output.rs").read_text(encoding="utf-8")
    for label in (
        "overall:", "stage:", "role:", "controller:", "service:", "wsl:",
        "podman_machine:", "kvm:", "tailscale:", "tailscale_serve:", "proxmox:",
        "cluster_joined:", "cluster_quorate:",
    ):
        if label not in output:
            fail(f"human status omits {label}")
    if "human_status_exposes_the_complete_mvp_contract" not in output:
        fail("CLI human status regression test is absent")

    client = (cli_root / "src" / "client.rs").read_text(encoding="utf-8")
    for marker in ("decode_status_response", "decode_operation_response", "PROTOCOL_SCHEMA_VERSION"):
        if marker not in client:
            fail(f"CLI protocol guard is absent: {marker}")

    request = (ROOT / "crates" / "gnx-protocol" / "src" / "request.rs").read_text(encoding="utf-8")
    body = re.search(r"pub enum Command\s*\{(?P<body>.*?)\}", request, re.S)
    if body is None:
        fail("protocol command enum is absent")
    named_pipe_commands = {
        item.strip().rstrip(",") for item in body.group("body").splitlines() if item.strip()
    }
    if named_pipe_commands != {"Status", "Configure"}:
        fail(f"Named Pipe command set differs: {sorted(named_pipe_commands)}")

    ns = {"w": "http://wixtoolset.org/schemas/v4/wxs"}
    package_root = ET.parse(ROOT / "installer" / "package.wxs").getroot()
    cli_components = package_root.findall(".//w:Component[@Id='CliComponent']", ns)
    if len(cli_components) != 1:
        fail("MSI must contain exactly one CliComponent")
    files = cli_components[0].findall("w:File[@Id='GnxCli']", ns)
    if len(files) != 1 or files[0].attrib.get("Name") != "gnx.exe" or files[0].attrib.get("KeyPath") != "yes":
        fail("MSI CLI file contract differs")
    environments = cli_components[0].findall("w:Environment[@Id='SystemPath']", ns)
    if len(environments) != 1:
        fail("MSI CLI PATH registration is absent")

    build = (ROOT / "installer" / "build.ps1").read_text(encoding="utf-8")
    for marker in ("'gnx-cli'", "target\\release\\gnx.exe", "-CliBinary $gnxCli"):
        if marker not in build:
            fail(f"installer CLI marker is absent: {marker}")

    print("cli-contract-validation: ok")


if __name__ == "__main__":
    main()
