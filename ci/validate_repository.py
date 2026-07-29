#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
import tomllib
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.1.17"
CRATES = {
    "gnx-cli": "gnx-cli",
    "gnx-protocol": "gnx-protocol",
    "gnx-service": "gnx-service",
    "gnx-host-preflight": "gnx-host-preflight",
}
WORKSPACE_MEMBERS = {f"crates/{directory}" for directory in CRATES}
INSTALLER_MODULES = {
    "dependencies.ps1",
    "contracts.ps1",
    "runtime.ps1",
    "rust.ps1",
    "msi.ps1",
    "bundle.ps1",
}
CLI_MODULES = {
    "client.rs",
    "commands/configure.rs",
    "commands/mod.rs",
    "commands/restart.rs",
    "commands/status.rs",
    "commands/version.rs",
    "error.rs",
    "main.rs",
    "output.rs",
}
PREFLIGHT_MODULES = {
    "checks.rs",
    "dependency.rs",
    "exit_codes.rs",
    "host_profile.rs",
    "journal.rs",
    "main.rs",
    "model.rs",
    "staging.rs",
    "windows.rs",
}
PROTOCOL_MODULES = {
    "lib.rs",
    "request.rs",
    "response.rs",
    "status.rs",
    "version.rs",
}
SERVICE_MODULES = {
    "ipc/mod.rs",
    "main.rs",
    "runtime/cluster/authorize_member.rs",
    "runtime/cluster/confirm_membership.rs",
    "runtime/cluster/mod.rs",
    "runtime/cluster/prepare_member.rs",
    "runtime/cluster/verify_member.rs",
    "runtime/control.rs",
    "runtime/error.rs",
    "runtime/host.rs",
    "runtime/machine.rs",
    "runtime/model.rs",
    "runtime/mod.rs",
    "runtime/payload.rs",
    "runtime/profile.rs",
    "runtime/proxmox.rs",
    "runtime/reconciler.rs",
    "runtime/remote/client.rs",
    "runtime/remote/limits.rs",
    "runtime/remote/mod.rs",
    "runtime/remote/operation.rs",
    "runtime/remote/transport.rs",
    "runtime/status.rs",
    "runtime/tailscale.rs",
    "runtime/tests.rs",
    "runtime/topology.rs",
    "secrets/mod.rs",
    "service/mod.rs",
    "state/mod.rs",
}


def fail(message: str) -> None:
    print(f"repository-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def cargo_version(path: Path) -> str:
    with path.open("rb") as handle:
        return tomllib.load(handle)["package"]["version"]


def rust_files(root: Path) -> set[str]:
    return {
        str(path.relative_to(root)).replace("\\", "/")
        for path in root.rglob("*.rs")
    }


def validate_versions() -> None:
    if (ROOT / "VERSION").read_text(encoding="utf-8").strip() != VERSION:
        fail("VERSION does not match the release")
    for directory, package_name in CRATES.items():
        manifest = ROOT / "crates" / directory / "Cargo.toml"
        if cargo_version(manifest) != VERSION:
            fail(f"crate version mismatch: {manifest.relative_to(ROOT)}")

    lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
    for package_name in CRATES.values():
        pattern = rf'(?s)\[\[package\]\]\nname = "{re.escape(package_name)}"\nversion = "{re.escape(VERSION)}"'
        if not re.search(pattern, lock):
            fail(f"Cargo.lock version mismatch for {package_name}")

    ns = {"w": "http://wixtoolset.org/schemas/v4/wxs"}
    package = ET.parse(ROOT / "installer" / "package.wxs").getroot().find("w:Package", ns)
    bundle = ET.parse(ROOT / "installer" / "bundle.wxs").getroot().find("w:Bundle", ns)
    if package is None or bundle is None:
        fail("WiX package or bundle root is absent")
    if package.attrib.get("Version") != VERSION or bundle.attrib.get("Version") != VERSION:
        fail("WiX versions do not match the release")

    build = (ROOT / "installer" / "build.ps1").read_text(encoding="utf-8")
    if f'$releaseVersion = "{VERSION}"' not in build:
        fail("installer build release version differs")


def validate_workspace() -> None:
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    if set(workspace["workspace"]["members"]) != WORKSPACE_MEMBERS:
        fail("workspace must contain the four 0.1.17 packages")
    if (ROOT / "crates" / "host-preflight").exists():
        fail("legacy crates/host-preflight directory remains")
    if (ROOT / ".github").exists():
        fail("hosted workflows remain outside the MVP scope")


def validate_module_sets() -> None:
    module_roots = (
        (ROOT / "crates" / "gnx-cli" / "src", CLI_MODULES, "CLI"),
        (ROOT / "crates" / "gnx-host-preflight" / "src", PREFLIGHT_MODULES, "preflight"),
        (ROOT / "crates" / "gnx-protocol" / "src", PROTOCOL_MODULES, "protocol"),
        (ROOT / "crates" / "gnx-service" / "src", SERVICE_MODULES, "service"),
    )
    for root, expected, label in module_roots:
        actual = rust_files(root)
        if actual != expected:
            fail(f"{label} module set differs: {sorted(actual ^ expected)}")

    forbidden = (
        ROOT / "crates" / "gnx-cli" / "src" / "pipe.rs",
        ROOT / "crates" / "gnx-service" / "src" / "pipe.rs",
        ROOT / "crates" / "gnx-service" / "src" / "secrets.rs",
        ROOT / "crates" / "gnx-service" / "src" / "state.rs",
        ROOT / "crates" / "gnx-service" / "src" / "runtime_gate.rs",
        ROOT / "crates" / "gnx-service" / "src" / "runtime" / "remote" / "process.rs",
    )
    for path in forbidden:
        if path.exists():
            fail(f"legacy source remains: {path.relative_to(ROOT)}")


def validate_architecture() -> None:
    cli_main = (ROOT / "crates" / "gnx-cli" / "src" / "main.rs").read_text(encoding="utf-8")
    if len(cli_main.splitlines()) > 35:
        fail("gnx-cli main.rs is no longer a thin composition root")
    for module in ("client", "commands", "error", "output"):
        if f"mod {module};" not in cli_main:
            fail(f"gnx-cli composition root omits {module}")

    service_main = (ROOT / "crates" / "gnx-service" / "src" / "main.rs").read_text(encoding="utf-8")
    if len(service_main.splitlines()) > 40 or "service::run()" not in service_main:
        fail("gnx-service main.rs is not a thin composition root")

    service = (ROOT / "crates" / "gnx-service" / "src" / "service" / "mod.rs").read_text(encoding="utf-8")
    if "RuntimeControl::start" not in service or "crate::ipc::serve" not in service:
        fail("service composition does not own runtime and IPC startup")

    ipc = (ROOT / "crates" / "gnx-service" / "src" / "ipc" / "mod.rs").read_text(encoding="utf-8")
    for forbidden in ("crate::runtime", "tailscale::", "proxmox::", "machine::"):
        if forbidden in ipc:
            fail(f"IPC bypasses the service/runtime boundary: {forbidden}")

    runtime_mod = (ROOT / "crates" / "gnx-service" / "src" / "runtime" / "mod.rs").read_text(encoding="utf-8")
    if "pub(crate) mod control;" not in runtime_mod:
        fail("runtime control facade is absent")
    if "reconciler::run(&status)" not in runtime_mod:
        fail("runtime facade does not delegate to the reconciler")

    protocol_lib = (ROOT / "crates" / "gnx-protocol" / "src" / "lib.rs").read_text(encoding="utf-8")
    for module in ("request", "response", "status", "version"):
        if f"mod {module};" not in protocol_lib:
            fail(f"protocol boundary omits {module}")

    for path in sorted((ROOT / "crates").rglob("*.rs")):
        lines = path.read_text(encoding="utf-8").splitlines()
        for index, line in enumerate(lines):
            if not line.lstrip().startswith("#["):
                continue
            cursor = index + 1
            while cursor < len(lines) and (
                not lines[cursor].strip() or lines[cursor].lstrip().startswith("#[")
            ):
                cursor += 1
            if cursor == len(lines):
                fail(f"dangling Rust attribute at {path.relative_to(ROOT)}:{index + 1}")


def validate_installer() -> None:
    module_root = ROOT / "installer" / "modules"
    actual = {path.name for path in module_root.glob("*.ps1")}
    if actual != INSTALLER_MODULES:
        fail(f"installer module set differs: {sorted(actual ^ INSTALLER_MODULES)}")
    build = (ROOT / "installer" / "build.ps1").read_text(encoding="utf-8")
    for name in INSTALLER_MODULES:
        if name not in build:
            fail(f"installer/build.ps1 does not load {name}")
    if "crates\\gnx-host-preflight" not in (ROOT / "installer" / "modules" / "contracts.ps1").read_text(encoding="utf-8"):
        fail("installer contracts still use the legacy preflight path")
    if '$dotnetToolManifest = Join-Path $repoRoot ".config\\dotnet-tools.json"' not in build:
        fail("installer does not pin the local .NET tool manifest")
    if "Unblock-File -LiteralPath $dotnetToolManifest -ErrorAction Stop" not in build:
        fail("installer does not clear Mark-of-the-Web from the tool manifest")


def validate_powershell_structure() -> None:
    for path in sorted((ROOT / "installer").rglob("*.ps1")):
        source = path.read_text(encoding="utf-8")
        stack: list[tuple[str, int]] = []
        pairs = {")": "(", "]": "[", "}": "{"}
        quote: str | None = None
        escaped = False
        comment = False
        for index, char in enumerate(source):
            if comment:
                if char == "\n":
                    comment = False
                continue
            if quote is not None:
                if escaped:
                    escaped = False
                elif char == "`":
                    escaped = True
                elif char == quote:
                    quote = None
                continue
            if char == "#":
                comment = True
            elif char in ("'", '"'):
                quote = char
            elif char in pairs.values():
                stack.append((char, index))
            elif char in pairs:
                if not stack or stack[-1][0] != pairs[char]:
                    fail(f"PowerShell delimiter mismatch in {path.relative_to(ROOT)}")
                stack.pop()
        if quote is not None or stack:
            fail(f"PowerShell source is structurally incomplete: {path.relative_to(ROOT)}")


def validate_delivery_records() -> None:
    required = {
        ROOT / ".AGENTS" / "agents" / "installer-recovery.md",
        ROOT / ".AGENTS" / "agents" / "cli-release-contract.md",
        ROOT / ".AGENTS" / "agents" / "cluster-membership.md",
        ROOT / ".AGENTS" / "tasks" / "RELEASE_0.1.17.md",
        ROOT / ".AGENTS" / "tasks" / "INSTALLER_RECOVERY.md",
        ROOT / ".AGENTS" / "tasks" / "MEMBER_JOIN.md",
        ROOT / "docs" / "ARCHITECTURE.md",
        ROOT / "docs" / "INSTALLER_RECOVERY.md",
        ROOT / "docs" / "MEMBER_MEMBERSHIP.md",
        ROOT / "docs" / "HOST_RESOURCE_PROFILE.md",
        ROOT / "docs" / "TARGET_0.2.md",
    }
    missing = [str(path.relative_to(ROOT)) for path in required if not path.is_file()]
    if missing:
        fail(f"0.1.17 delivery records are absent: {missing}")


def main() -> None:
    validate_versions()
    validate_workspace()
    validate_module_sets()
    validate_architecture()
    validate_installer()
    validate_powershell_structure()
    validate_delivery_records()
    print("repository-validation: ok")


if __name__ == "__main__":
    main()
