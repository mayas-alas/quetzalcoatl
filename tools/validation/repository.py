#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXPECTED_MEMBERS = [
    "apps/gnx",
    "apps/gnx-service",
    "apps/gnx-bootstrap",
    "crates/gnx-contracts",
]
EXPECTED_DOCS = {
    "README.md",
    "ARCHITECTURE.md",
    "CONTRACTS.md",
    "OPERATIONS.md",
    "TROUBLESHOOTING.md",
    "VALIDATION.md",
    "AGENT_WORKFLOW.md",
    "AGENT_AUDIT.md",
    "DASHBOARD.md",
}
EXPECTED_SERVICES = {
    "platform/services/forgejo/compose.yml",
    "platform/services/forgejo/serve.json",
    "platform/services/garage/compose.yml",
    "platform/services/garage/serve.json",
    "platform/services/runner/compose.yml",
    "platform/services/service/compose.yml",
    "platform/services/service/serve.json",
    "platform/services/freellmapi/compose.yml",
    "platform/services/freellmapi/policy.json",
    "platform/services/freellmapi/serve.json",
    "platform/services/omniroute/compose.yml",
    "platform/services/omniroute/policy.json",
    "platform/services/omniroute/serve.json",
}
IGNORED_ROOTS = {".git", ".wix", "target", ".kilo", ".AGENTS/agentA"}
FORBIDDEN_NAME = re.compile(
    r"(?:_v\d+|-v\d+|(?:^|[-_.])(old|legacy|new|final|buildfix)(?:[-_.]|$))",
    re.IGNORECASE,
)
SECRET_SHAPED_TEXT = re.compile(r"freellmapi-[a-f0-9]{32,}", re.IGNORECASE)
SCANNED_TEXT_SUFFIXES = {
    ".json",
    ".md",
    ".ps1",
    ".py",
    ".rs",
    ".sh",
    ".tf",
    ".toml",
    ".txt",
    ".yaml",
    ".yml",
}


def fail(message: str) -> None:
    print(f"repository-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    members = workspace.get("workspace", {}).get("members", [])
    if members != EXPECTED_MEMBERS:
        fail(f"workspace members differ: {members!r}")

    package_manifests = sorted(
        path.relative_to(ROOT).as_posix()
        for root in (ROOT / "apps", ROOT / "crates")
        for path in root.rglob("Cargo.toml")
    )
    expected_manifests = sorted(f"{member}/Cargo.toml" for member in EXPECTED_MEMBERS)
    if package_manifests != expected_manifests:
        fail(f"Cargo package inventory differs: {package_manifests!r}")

    if (ROOT / "ci").exists() and any((ROOT / "ci").rglob("*")):
        fail("parallel ci/ validation entry point remains")
    if (ROOT / "installer" / "docs").exists() and any(
        (ROOT / "installer" / "docs").rglob("*")
    ):
        fail("installer-specific documentation duplicates docs/operations")

    actual_docs = {
        path.relative_to(ROOT / "docs").as_posix()
        for path in (ROOT / "docs").rglob("*")
        if path.is_file()
    }
    if actual_docs != EXPECTED_DOCS:
        fail(
            "documentation taxonomy differs: "
            f"missing={sorted(EXPECTED_DOCS - actual_docs)!r} "
            f"extra={sorted(actual_docs - EXPECTED_DOCS)!r}"
        )

    for path in ROOT.rglob("*"):
        if not path.is_file() or path.suffix.lower() not in SCANNED_TEXT_SUFFIXES:
            continue
        relative = path.relative_to(ROOT)
        if relative.parts and relative.parts[0] in IGNORED_ROOTS:
            continue
        try:
            source = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        if SECRET_SHAPED_TEXT.search(source):
            fail(f"secret-shaped FreeLLMAPI credential in {relative.as_posix()}")

    for path in ROOT.rglob("*"):
        relative = path.relative_to(ROOT)
        if relative.parts and relative.parts[0] in IGNORED_ROOTS:
            continue
        for part in relative.parts:
            if FORBIDDEN_NAME.search(part):
                fail(f"non-semantic versioned or transitional name remains: {relative}")

    agents = sorted(
        path.relative_to(ROOT / ".AGENTS").as_posix()
        for path in (ROOT / ".AGENTS").rglob("*")
        if path.is_file() and not path.relative_to(ROOT / ".AGENTS").parts[0] == "agentA"
    )
    expected_agents = sorted(
        [
            "README.md",
            "SCOPE.md",
            "WORKSTREAMS.md",
            "TRACKER.md",
            "EVIDENCE.md",
            "SPEC.md",
            "COORDINATOR.md",
            "CAPACITY.md",
            "gauntlet/BOARD.md",
            "gauntlet/GAUNTLET.md",
            "gauntlet/MODEL.md",
            "gauntlet/README.md",
        ]
    )
    if agents != expected_agents:
        fail(f".AGENTS live inventory differs: {agents!r}")

    if (ROOT / "VERSION").exists():
        fail("redundant root VERSION file remains")
    if (ROOT / "icons").exists():
        fail("root icons directory remains outside installer/assets/branding")
    if (ROOT / "assets").exists():
        fail("root assets directory remains outside installer ownership")

    actual_services = {
        path.relative_to(ROOT).as_posix()
        for path in (ROOT / "platform" / "services").rglob("*")
        if path.is_file()
    }
    if actual_services != EXPECTED_SERVICES:
        fail(
            "platform services inventory differs: "
            f"missing={sorted(EXPECTED_SERVICES - actual_services)!r} "
            f"extra={sorted(actual_services - EXPECTED_SERVICES)!r}"
        )

    build_rs = (ROOT / "apps" / "gnx" / "build.rs").read_text(encoding="utf-8")
    if '"0.2.0"' in build_rs or 'VALUE "CompanyName", "Quetzalcoatl\\0"' in build_rs:
        fail("PE metadata contains duplicated version or company identity")

    security_gate = (ROOT / "tools" / "security.ps1").read_text(encoding="utf-8")
    for marker in ("cargo audit --version", "0.22.2", "cargo audit --deny warnings"):
        if marker not in security_gate:
            fail(f"security gate omits {marker!r}")
    dependency_lock = (
        ROOT / "installer" / "dependencies.lock.json"
    ).read_text(encoding="utf-8")
    if dependency_lock.count('"authenticode"') != 3:
        fail("installer dependency Authenticode policy inventory differs")

    for path in (ROOT / "apps").rglob("*.rs"):
        source = path.read_text(encoding="utf-8")
        if "#[path" in source:
            fail(f"Rust module uses cross-folder #[path] wiring: {path.relative_to(ROOT)}")
        if path.name.endswith("_tests.rs"):
            continue
        test_module = False
        pending_test_module = False
        for number, line in enumerate(source.splitlines(), start=1):
            stripped = line.strip()
            if stripped == "#[cfg(test)]":
                pending_test_module = True
                continue
            if pending_test_module and re.match(r"(?:pub(?:\([^)]*\))?\s+)?mod\s+tests\s*\{", stripped):
                test_module = True
            pending_test_module = False
            if test_module:
                continue
            if re.match(r"\s*(?:pub\(crate\)\s+)?use\s+.+::\*;", line):
                fail(
                    "production Rust module uses a broad glob import: "
                    f"{path.relative_to(ROOT)}:{number}"
                )

    print("repository-validation: ok")


if __name__ == "__main__":
    main()
