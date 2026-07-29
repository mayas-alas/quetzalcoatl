#!/usr/bin/env python3
from __future__ import annotations

import json
import hashlib
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def fail(message: str) -> None:
    print(f"contract-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def require(source: str, markers: tuple[str, ...], owner: str) -> None:
    for marker in markers:
        if marker not in source:
            fail(f"{owner} omits {marker!r}")


def main() -> None:
    release = tomllib.loads((ROOT / "release" / "manifest.toml").read_text(encoding="utf-8"))
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    version = release["version"]
    if version != "0.2.12":
        fail("release manifest must identify 0.2.12")
    if workspace["workspace"]["package"]["version"] != version:
        fail("workspace package version differs from the release manifest")
    identities = release["identities"]
    guid = re.compile(
        r"^\{[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-"
        r"[0-9A-F]{4}-[0-9A-F]{12}\}$"
    )
    if any(not guid.fullmatch(value) for value in identities.values()):
        fail("release identity is not an uppercase braced GUID")
    if identities["upgrade_code"] != "{47D5BD44-D061-407B-913B-47D17EC3BEA9}":
        fail("MSI upgrade family changed")
    if identities["bundle_upgrade_code"] != "{10B764B2-36AE-4911-A8C8-2F1A2A963769}":
        fail("Burn upgrade family changed")
    for current, previous in (
        ("product_code", "previous_product_code"),
        ("package_code", "previous_package_code"),
        ("bundle_id", "previous_bundle_id"),
    ):
        if identities[current] == identities[previous]:
            fail(f"release reuses {current} from the previous package")
    if (
        identities["previous_product_code"]
        != "{EC86FFE0-5AD0-47C9-8959-74F543BA767C}"
        or identities["previous_package_code"]
        != "{1F32843D-AE3C-453C-8A16-1F0DFE8E704C}"
        or identities["previous_bundle_id"]
        != "{F0BFF877-A301-4F13-85F7-3F9E4429C6CC}"
    ):
        fail("0.2.12 does not identify the physically rejected 0.2.11 package")
    package = workspace["workspace"]["package"]
    if package.get("license") != "AGPL-3.0-only":
        fail("workspace product license must be AGPL-3.0-only")
    if package.get("authors") != ["Hector AB and other contributors"]:
        fail("workspace author metadata differs from the product notice")
    license_hash = hashlib.sha256((ROOT / "LICENSE").read_bytes()).hexdigest().upper()
    if license_hash != "D8A6CC31ABC16B6748C7A21F21611F5A1EC33F67D22CA23D7DA1C19B95496BEE":
        fail("root LICENSE differs from the canonical AGPL-3.0-only text")
    notice = (ROOT / "NOTICE").read_text(encoding="utf-8")
    if "Copyright (c) 2008-2020 Hector AB and other contributors" not in notice:
        fail("root NOTICE omits the exact product copyright")

    expected_contracts = {
        "protocol_schema": 2,
        "persisted_state_schema": 2,
        "host_profile_schema": 1,
        "payload_contract": 5,
        "runtime_generation": "proxmox-cluster-v2",
    }
    if release["contracts"] != expected_contracts:
        fail(f"release contracts differ: {release['contracts']!r}")

    expected_paths = {
        "gnx": "apps/gnx",
        "gnx_service": "apps/gnx-service",
        "gnx_bootstrap": "apps/gnx-bootstrap",
        "gnx_contracts": "crates/gnx-contracts",
        "branding": "installer/assets/branding",
        "runtime": "runtime",
        "installer": "installer",
    }
    if release["paths"] != expected_paths:
        fail(f"release path map differs: {release['paths']!r}")

    migration = (ROOT / "crates" / "gnx-contracts" / "src" / "migration.rs").read_text(
        encoding="utf-8"
    )
    require(
        migration,
        (
            "PROTOCOL_SCHEMA_VERSION: u8 = 2",
            "PERSISTED_STATE_SCHEMA_VERSION: u8 = 2",
            "HOST_PROFILE_SCHEMA_VERSION: u8 = 1",
            "RUNTIME_PAYLOAD_CONTRACT: u8 = 5",
            'RUNTIME_GENERATION: &str = "proxmox-cluster-v2"',
        ),
        "shared migration contract",
    )

    fixtures = ROOT / "tests" / "contracts" / "fixtures"
    request = json.loads((fixtures / "request-status-schema-2.json").read_text(encoding="utf-8"))
    status = json.loads((fixtures / "status-member-ready-schema-2.json").read_text(encoding="utf-8"))
    if request != {"command": "status"} or status.get("schema_version") != 2:
        fail("IPC compatibility fixtures differ from the schema-2 command contract")
    if status.get("role") != "member" or status.get("overall") != "ready":
        fail("ready-member fixture differs")

    cli = (ROOT / "apps" / "gnx" / "src" / "commands" / "mod.rs").read_text(encoding="utf-8")
    require(
        cli,
        (
            'command == "version"',
            'command == "--version"',
            'command == "-V"',
            'parse(&["-v"]).is_err()',
        ),
        "CLI version contract",
    )

    recovery = (
        ROOT / "apps" / "gnx-bootstrap" / "src" / "recovery" / "mod.rs"
    ).read_text(encoding="utf-8")
    require(
        recovery,
        (
            "InstallOperation::Upgrade",
            "InstallOperation::Repair",
            '"0.1.17"',
            'env!("CARGO_PKG_VERSION")',
            "migrates_a_previous_release_journal_into_upgrade_operation",
            "migrates_the_incomplete_0_2_0_journal_into_upgrade_operation",
            "migrates_the_0_2_1_journal_into_upgrade_operation",
            "migrates_the_0_2_4_journal_into_upgrade_operation",
            "a_repair_request_is_explicit_and_keeps_the_current_checkpoint",
        ),
        "installer recovery contract",
    )

    print("contract-validation: ok")


if __name__ == "__main__":
    main()
