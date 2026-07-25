#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
import tomllib
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.1.13"
CRATES = {
    "gnx-cli": "gnx-cli",
    "gnx-protocol": "gnx-protocol",
    "gnx-service": "gnx-service",
    "host-preflight": "gnx-host-preflight",
}
INSTALLER_MODULES = {
    "dependencies.ps1",
    "contracts.ps1",
    "runtime.ps1",
    "rust.ps1",
    "msi.ps1",
    "bundle.ps1",
}
RUNTIME_MODULES = {
    "error.rs",
    "host.rs",
    "machine.rs",
    "model.rs",
    "mod.rs",
    "payload.rs",
    "proxmox.rs",
    "reconciler.rs",
    "status.rs",
    "tailscale.rs",
    "tests.rs",
    "topology.rs",
    "remote/client.rs",
    "remote/limits.rs",
    "remote/mod.rs",
    "remote/operation.rs",
    "remote/transport.rs",
}


def fail(message: str) -> None:
    print(f"repository-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def cargo_version(path: Path) -> str:
    with path.open("rb") as handle:
        return tomllib.load(handle)["package"]["version"]


def validate_versions() -> None:
    if (ROOT / "VERSION").read_text(encoding="utf-8").strip() != VERSION:
        fail("VERSION does not match the release")
    for directory, package_name in CRATES.items():
        path = ROOT / "crates" / directory / "Cargo.toml"
        if cargo_version(path) != VERSION:
            fail(f"crate version mismatch: {path}")

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
    if package.attrib.get("ProductCode") == "{621E6E65-5BB1-5495-A887-F3AF3AA57125}":
        fail("0.1.13 reused the 0.1.12 ProductCode")

    build = (ROOT / "installer" / "build.ps1").read_text(encoding="utf-8")
    if f'$releaseVersion = "{VERSION}"' not in build:
        fail("installer build release version differs")
    if "runtime\\payload-v1" in build or "runtime\\payload-v2" in build:
        fail("installer still references a legacy runtime payload")


def validate_installer_modules() -> None:
    module_root = ROOT / "installer" / "modules"
    actual = {path.name for path in module_root.glob("*.ps1")}
    if actual != INSTALLER_MODULES:
        fail(f"installer module set differs: {sorted(actual ^ INSTALLER_MODULES)}")
    build = (ROOT / "installer" / "build.ps1").read_text(encoding="utf-8")
    if re.search(r"(?m)^function\s+", build):
        fail("installer/build.ps1 still contains top-level function implementations")
    for name in INSTALLER_MODULES:
        if name not in build:
            fail(f"installer/build.ps1 does not load {name}")
    if "$installerRoot = $PSScriptRoot" not in build:
        fail("installer/build.ps1 does not define the shared installer root")
    if "Test-RuntimePayloadSource" not in build or "Build-RustReleaseArtifacts" not in build:
        fail("installer entry point does not invoke runtime and Rust modules")
    if '$dotnetToolManifest = Join-Path $repoRoot ".config\\dotnet-tools.json"' not in build:
        fail("installer does not pin the local .NET tool manifest path")
    if "Unblock-File -LiteralPath $dotnetToolManifest -ErrorAction Stop" not in build:
        fail("installer does not clear Mark-of-the-Web from the .NET tool manifest")
    if "dotnet tool restore --tool-manifest $dotnetToolManifest" not in build:
        fail("installer does not restore WiX from the explicit tool manifest")
    for path in sorted(module_root.glob("*.ps1")):
        source = path.read_text(encoding="utf-8")
        if "$PSScriptRoot" in source:
            fail(f"installer module uses its own PSScriptRoot: {path.name}")


def validate_rust_module_boundaries() -> None:
    runtime = ROOT / "crates" / "gnx-service" / "src" / "runtime"
    actual = {
        str(path.relative_to(runtime)).replace("\\", "/")
        for path in runtime.rglob("*.rs")
    }
    if actual != RUNTIME_MODULES:
        fail(f"runtime module set differs: {sorted(actual ^ RUNTIME_MODULES)}")
    if (runtime.parent / "runtime_gate.rs").exists():
        fail("legacy runtime_gate.rs remains")

    for path in sorted(runtime.rglob("*.rs")):
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

    mod_source = (runtime / "mod.rs").read_text(encoding="utf-8")
    if "fn run_inner(" in mod_source:
        fail("runtime orchestration remains in mod.rs")
    if "reconciler::run(&status)" not in mod_source:
        fail("runtime facade does not delegate to reconciler")
    if "pub(super) fn run(" not in (runtime / "reconciler.rs").read_text(encoding="utf-8"):
        fail("reconciler entry point is absent")

    model = (runtime / "model.rs").read_text(encoding="utf-8")
    if not re.search(r"#\[derive\(Clone, Copy, Debug\)\]\s+pub\(super\) enum Component", model):
        fail("runtime Component lost required derives")
    if "struct GateError" in model or "struct GateError" not in (runtime / "error.rs").read_text(encoding="utf-8"):
        fail("runtime error boundary is not separated")

    remote_mod = (runtime / "remote" / "mod.rs").read_text(encoding="utf-8")
    for module in ("client", "operation", "transport"):
        if f"pub(in crate::runtime) use {module}::*;" not in remote_mod:
            fail(f"remote module does not re-export {module}")
    if (runtime / "remote" / "process.rs").exists():
        fail("legacy remote/process.rs remains")



def validate_cli_boundary() -> None:
    if not (ROOT / "ci" / "validate_cli_contract.py").is_file():
        fail("CLI contract validator is absent")
    for path in (
        ROOT / ".AGENTS" / "agents" / "runtime-modularization.md",
        ROOT / ".AGENTS" / "tasks" / "PROXMOX_CLUSTER_RUNTIME.md",
        ROOT / ".AGENTS" / "tasks" / "RELEASE_0.1.11.md",
        ROOT / ".AGENTS" / "tasks" / "RELEASE_0.1.12.md",
    ):
        if path.exists():
            fail(f"legacy delivery record remains: {path.relative_to(ROOT)}")


def validate_powershell_structure() -> None:
    for path in sorted((ROOT / "installer").rglob("*.ps1")):
        source = path.read_text(encoding="utf-8")
        stack: list[tuple[str, int]] = []
        pairs = {")": "(", "]": "[", "}": "{"}
        openers = set(pairs.values())
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
            elif char in openers:
                stack.append((char, index))
            elif char in pairs:
                if not stack or stack[-1][0] != pairs[char]:
                    fail(f"PowerShell delimiter mismatch in {path.relative_to(ROOT)}")
                stack.pop()
        if quote is not None or stack:
            fail(f"PowerShell source is structurally incomplete: {path.relative_to(ROOT)}")
        for line_no, line in enumerate(source.splitlines(), start=1):
            if line.rstrip().endswith("`") and line != line.rstrip():
                fail(f"PowerShell continuation has trailing whitespace: {path.relative_to(ROOT)}:{line_no}")

def validate_scope() -> None:
    if (ROOT / ".github").exists():
        fail("hosted workflow infrastructure is outside the 0.1.13 MVP scope")
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    if set(workspace["workspace"]["members"]) != {
        "crates/gnx-cli",
        "crates/gnx-protocol",
        "crates/gnx-service",
        "crates/host-preflight",
    }:
        fail("workspace must remain at the four existing crates")
    required_agent_docs = {
        ROOT / ".AGENTS" / "agents" / "runtime-transport.md",
        ROOT / ".AGENTS" / "agents" / "reconciler-recovery.md",
        ROOT / ".AGENTS" / "agents" / "release-integrity.md",
        ROOT / ".AGENTS" / "agents" / "cli-contract.md",
        ROOT / ".AGENTS" / "tasks" / "RELEASE_0.1.13.md",
        ROOT / ".AGENTS" / "tasks" / "CLI_CONTRACT_AUDIT.md",
        ROOT / ".AGENTS" / "tasks" / "REMOTE_EXECUTION_REMEDIATION.md",
        ROOT / ".AGENTS" / "tasks" / "RECONCILER_RECOVERY.md",
    }
    missing = [str(path.relative_to(ROOT)) for path in required_agent_docs if not path.is_file()]
    if missing:
        fail(f"0.1.13 agent delivery records are absent: {missing}")


def main() -> None:
    validate_versions()
    validate_installer_modules()
    validate_rust_module_boundaries()
    validate_cli_boundary()
    validate_powershell_structure()
    validate_scope()
    print("repository-validation: ok")


if __name__ == "__main__":
    main()
