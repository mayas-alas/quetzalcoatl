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
    "VALIDATION.md",
}
IGNORED_ROOTS = {".git", ".wix", "target"}
FORBIDDEN_NAME = re.compile(
    r"(?:_v\d+|-v\d+|(?:^|[-_.])(old|legacy|new|final|buildfix)(?:[-_.]|$))",
    re.IGNORECASE,
)


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
        relative = path.relative_to(ROOT)
        if relative.parts and relative.parts[0] in IGNORED_ROOTS:
            continue
        for part in relative.parts:
            if FORBIDDEN_NAME.search(part):
                fail(f"non-semantic versioned or transitional name remains: {relative}")

    agents = sorted(
        path.relative_to(ROOT / ".AGENTS").as_posix()
        for path in (ROOT / ".AGENTS").rglob("*")
        if path.is_file()
    )
    expected_agents = sorted(
        [
            "README.md",
            "SCOPE.md",
            "WORKSTREAMS.md",
            "TRACKER.md",
            "EVIDENCE.md",
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

    build_rs = (ROOT / "apps" / "gnx" / "build.rs").read_text(encoding="utf-8")
    if '"0.2.0"' in build_rs or 'VALUE "CompanyName", "Quetzalcoatl\\0"' in build_rs:
        fail("PE metadata contains duplicated version or company identity")

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
