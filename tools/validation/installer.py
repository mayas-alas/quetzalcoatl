#!/usr/bin/env python3
from __future__ import annotations

import sys
import struct
import json
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
INSTALLER = ROOT / "installer"


def fail(message: str) -> None:
    print(f"installer-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def local_name(element: ET.Element) -> str:
    return element.tag.rsplit("}", 1)[-1]


def png_dimensions(path: Path) -> tuple[int, int]:
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        fail(f"branding asset is not a valid PNG: {path.relative_to(ROOT)}")
    return struct.unpack(">II", data[16:24])


def main() -> None:
    bundle = ET.parse(INSTALLER / "source" / "bundle.wxs").getroot()
    product = ET.parse(INSTALLER / "source" / "product.wxs").getroot()
    theme = ET.parse(INSTALLER / "assets" / "wixstdba-theme.xml").getroot()
    dependency_lock = json.loads(
        (INSTALLER / "dependencies.lock.json").read_text(encoding="utf-8")
    )
    dependency_ids = [artifact["id"] for artifact in dependency_lock["artifacts"]]
    if dependency_ids != ["winsw", "wsl", "podman"]:
        fail(f"installer dependency inventory differs: {dependency_ids!r}")

    exe_packages = {
        node.attrib["Id"]: node
        for node in bundle.iter()
        if local_name(node) == "ExePackage"
    }
    expected = {
        "PrepareWsl": (
            "prepare-wsl --operation install --format json",
            "prepare-wsl --operation repair --format json",
        ),
        "InstallWsl": (
            "install-wsl --operation install --format json",
            "install-wsl --operation repair --format json",
        ),
        "InstallPodman": (
            "install-podman --operation install --format json",
            "install-podman --operation repair --format json",
        ),
        "ValidateHost": (
            "--operation install --format json",
            "--operation repair --format json",
        ),
    }
    if set(exe_packages) != set(expected):
        fail(f"bundle helper inventory differs: {sorted(exe_packages)!r}")
    for package_id, (install, repair) in expected.items():
        node = exe_packages[package_id]
        if (
            node.attrib.get("SourceFile") != "$(var.GnxBootstrap)"
            or node.attrib.get("InstallArguments") != install
            or node.attrib.get("RepairArguments") != repair
            or node.attrib.get("RepairCondition") != "1"
        ):
            fail(f"closed maintenance operation differs for {package_id}")

    msi_packages = [
        node for node in bundle.iter() if local_name(node) == "MsiPackage"
    ]
    if len(msi_packages) != 1:
        fail("bundle must contain one internal product MSI")
    if (
        msi_packages[0].attrib.get("Id") != "QuetzalcoatlProduct"
        or msi_packages[0].attrib.get("Visible") != "no"
    ):
        fail("internal product MSI must be hidden behind the sole Setup ARP entry")
    if not any(
        local_name(node) == "Button" and node.attrib.get("Name") == "RepairButton"
        for node in theme.iter()
    ):
        fail("bootstrapper repair action is absent")

    components = {
        node.attrib.get("Id"): node
        for node in product.iter()
        if local_name(node) == "Component"
    }
    for component_id, file_id in (
        ("CliComponent", "GnxCli"),
        ("TrayComponent", "GnxTray"),
        ("GnxServiceBinaryComponent", "GnxService"),
    ):
        component = components.get(component_id)
        files = [] if component is None else [
            node for node in component if local_name(node) == "File"
        ]
        if len(files) != 1 or files[0].attrib.get("Id") != file_id or files[0].attrib.get("KeyPath") != "yes":
            fail(f"MSI key path differs for {component_id}")

    tray_launch = next(
        (
            node
            for node in product.iter()
            if local_name(node) == "CustomAction"
            and node.attrib.get("Id") == "LaunchQuetzalcoatlTray"
        ),
        None,
    )
    if (
        tray_launch is None
        or tray_launch.attrib.get("FileRef") != "GnxTray"
        or tray_launch.attrib.get("ExeCommand") != "--launch-detached"
        or tray_launch.attrib.get("Execute") != "immediate"
        or tray_launch.attrib.get("Impersonate") != "yes"
        or tray_launch.attrib.get("Return") != "check"
        or "Directory" in tray_launch.attrib
    ):
        fail("tray launch must use the checked detached-launch contract")
    tray_shortcut = next(
        (
            node
            for node in product.iter()
            if local_name(node) == "Shortcut"
            and node.attrib.get("Id") == "TrayStartupShortcut"
        ),
        None,
    )
    if (
        tray_shortcut is None
        or tray_shortcut.attrib.get("WorkingDirectory") != "SystemFolder"
    ):
        fail("startup tray must not use the MSI-owned product directory as cwd")
    service_config = ET.parse(
        INSTALLER / "source" / "Quetzalcoatl.Service.xml"
    ).getroot()
    service_working_directory = service_config.findtext("workingdirectory")
    if service_working_directory != r"%ProgramData%\Quetzalcoatl\Installer":
        fail("service working directory must stay outside the MSI-owned product tree")
    if (
        service_config.findtext("stopexecutable") != r"%BASE%\gnx-service.exe"
        or service_config.findtext("stoparguments") != "--stop-managed-machine"
        or service_config.find("startarguments") is None
    ):
        fail("service stop must release the managed Podman machine without deleting it")

    payload_validator = next(
        (
            node
            for node in product.iter()
            if local_name(node) == "CustomAction"
            and node.attrib.get("Id") == "ValidateInstalledPayload"
        ),
        None,
    )
    if payload_validator is None or {
        key: payload_validator.attrib.get(key)
        for key in ("FileRef", "ExeCommand", "Execute", "Impersonate", "Return")
    } != {
        "FileRef": "GnxService",
        "ExeCommand": "--validate-installation",
        "Execute": "deferred",
        "Impersonate": "no",
        "Return": "check",
    }:
        fail("MSI installed-payload validation action differs")
    payload_sequence = next(
        (
            node
            for node in product.iter()
            if local_name(node) == "Custom"
            and node.attrib.get("Action") == "ValidateInstalledPayload"
        ),
        None,
    )
    if (
        payload_sequence is None
        or payload_sequence.attrib.get("After") != "InstallFiles"
        or payload_sequence.attrib.get("Condition") != 'NOT (REMOVE = "ALL")'
    ):
        fail("MSI payload validation is not sequenced before service start")

    build = (INSTALLER / "build.ps1").read_text(encoding="utf-8")
    rust_build = (INSTALLER / "modules" / "rust.ps1").read_text(encoding="utf-8")
    for marker in (
        "release\\manifest.toml",
        "source\\product.wxs",
        "source\\bundle.wxs",
        "extensions\\deterministic-bundle",
        "Test-MaintenanceContract",
        "Test-MsiPayloadCoherence",
        "Test-InstalledMsiIdentity",
        "SetLastWriteTimeUtc",
        "runtime-payload",
    ):
        if marker not in build:
            fail(f"release entry point omits {marker!r}")
    if "link-arg=/Brepro" not in rust_build:
        fail("release Rust artifacts are not linked reproducibly")
    service_main = (ROOT / "apps" / "gnx-service" / "src" / "main.rs").read_text(
        encoding="utf-8"
    )
    service_installation = (
        ROOT / "apps" / "gnx-service" / "src" / "application" / "installation.rs"
    ).read_text(encoding="utf-8")
    bootstrap_checks = (
        ROOT / "apps" / "gnx-bootstrap" / "src" / "host" / "checks.rs"
    ).read_text(encoding="utf-8")
    for marker in (
        "--validate-installation",
        "ValidateInstallation",
        "--stop-managed-machine",
        "service_shutdown::signal()",
    ):
        if marker not in service_main:
            fail(f"service installed-payload mode omits {marker!r}")
    service_shutdown = (
        ROOT
        / "apps"
        / "gnx-service"
        / "src"
        / "infrastructure"
        / "service_shutdown.rs"
    ).read_text(encoding="utf-8")
    for marker in ("CreateEventW", "WaitForSingleObject", "OpenEventW", "SetEvent"):
        if marker not in service_shutdown:
            fail(f"bounded service shutdown omits {marker!r}")
    for marker in ("load_payload_files()", "load_machine_image()", "installed_machine_image"):
        if marker not in service_installation:
            fail(f"installed-payload validation omits {marker!r}")
    for marker in (
        "feature_output_is_enabled",
        "decode_windows_text",
        "dism_enabled_state_accepts_utf16le_and_utf8",
        "dism_enabled_state_accepts_an_oem_banner",
        "pending_temp_deletions_do_not_force_a_reboot",
        "pending_file_replacements_still_require_a_reboot",
    ):
        if marker not in bootstrap_checks:
            fail(f"Windows feature-state decoding omits {marker!r}")
    if "operations" in "\n".join(
        line for line in build.splitlines() if "Copy-Item" in line
    ):
        fail("installer stages repository-owned operations as installed payload")

    tray = (ROOT / "apps" / "gnx" / "src" / "bin" / "gnx-tray.rs").read_text(
        encoding="utf-8"
    )
    for marker in (
        "MENU_STATUS",
        "MENU_VERSION",
        "MENU_CONNECT",
        '"Conectar"',
        '"Versión: {}"',
        "PveUrl::parse",
    ):
        if marker not in tray:
            fail(f"tray contract omits {marker!r}")
    contracts = (ROOT / "crates" / "gnx-contracts" / "src" / "status.rs").read_text(
        encoding="utf-8"
    )
    for marker in ('strip_prefix("https://")', 'ends_with(".ts.net")', "localhost"):
        if marker not in contracts:
            fail(f"shared PVE URL contract omits {marker!r}")

    for asset in (
        INSTALLER / "assets" / "branding" / "icon.ico",
        INSTALLER / "assets" / "branding" / "icon.png",
        INSTALLER / "assets" / "branding" / "installer-banner.png",
        INSTALLER / "assets" / "wixstdba-logo.png",
        INSTALLER / "assets" / "wixstdba-side.png",
        INSTALLER / "assets" / "wixstdba-theme.xml",
    ):
        if not asset.is_file() or asset.stat().st_size == 0:
            fail(f"branding asset is absent: {asset.relative_to(ROOT)}")

    expected_dimensions = {
        INSTALLER / "assets" / "branding" / "icon.png": (128, 128),
        INSTALLER / "assets" / "wixstdba-logo.png": (64, 64),
        INSTALLER / "assets" / "wixstdba-side.png": (165, 312),
    }
    for asset, dimensions in expected_dimensions.items():
        if png_dimensions(asset) != dimensions:
            fail(
                f"branding dimensions differ for {asset.relative_to(ROOT)}: "
                f"{png_dimensions(asset)!r}"
            )
    image_controls = [
        node for node in theme.iter() if local_name(node) == "ImageControl"
    ]
    side = next(
        (node for node in image_controls if node.attrib.get("ImageFile") == "logoside.png"),
        None,
    )
    if side is None or (side.attrib.get("Width"), side.attrib.get("Height")) != (
        "165",
        "312",
    ):
        fail("Burn sidebar control does not match the 165x312 derived banner")

    product_package = next(
        node for node in product.iter() if local_name(node) == "Package"
    )
    burn_bundle = next(node for node in bundle.iter() if local_name(node) == "Bundle")
    if product_package.attrib.get("Manufacturer") != "Hector AB":
        fail("MSI manufacturer differs from the product copyright owner")
    if burn_bundle.attrib.get("Manufacturer") != "Hector AB":
        fail("Burn manufacturer differs from the product copyright owner")
    license_owner = next(
        (
            node
            for node in bundle.iter()
            if node.attrib.get("LicenseUrl")
        ),
        None,
    )
    if (
        license_owner is None
        or license_owner.attrib.get("LicenseUrl")
        != "https://www.gnu.org/licenses/agpl-3.0.html"
    ):
        fail("Burn does not expose the AGPLv3 license")
    license_names = {
        node.attrib.get("Name")
        for node in product.iter()
        if local_name(node) == "File"
    }
    for required in (
        "AGPL-3.0.txt",
        "NOTICE.txt",
        "THIRD_PARTY_NOTICES.md",
        "WinSW.txt",
        "WiX.txt",
    ):
        if required not in license_names:
            fail(f"MSI omits legal file {required}")

    print("installer-validation: ok")


if __name__ == "__main__":
    main()
