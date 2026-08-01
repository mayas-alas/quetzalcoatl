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

    build_script = (INSTALLER / "build.ps1").read_text(encoding="utf-8")
    for wrapper_contract in (
        '$serviceWrapper = Join-Path $outputRoot "Quetzalcoatl.Service.exe"',
        "Copy-Item -LiteralPath $artifacts.winsw -Destination $serviceWrapper -Force",
        "Invoke-AuthenticodeSign `\n            -Path $serviceWrapper",
        '-d "WinSW=$serviceWrapper"',
        "-ServiceWrapper $serviceWrapper",
    ):
        if wrapper_contract not in build_script:
            fail(f"service wrapper release contract omits {wrapper_contract!r}")
    if "Copy-PlatformPayload `" not in build_script:
        fail("installer must stage the platform exclusively from its lock")
    if "Copy-Item -LiteralPath (Join-Path $repoRoot 'platform')" in build_script:
        fail("installer must not copy the unlocked platform tree")

    exe_packages = {
        node.attrib["Id"]: node
        for node in bundle.iter()
        if local_name(node) == "ExePackage"
    }
    expected = {
        "PrepareQaTrust": (
            'prepare-qa-trust --root-certificate "[WixBundleExecutePackageCacheFolder]\\gnx-qa-root.cer" --root-sha256 $(var.QaRootSha256) --publisher-certificate "[WixBundleExecutePackageCacheFolder]\\gnx-qa-publisher.cer" --publisher-sha256 $(var.QaPublisherSha256) --operation install --format json',
            'prepare-qa-trust --root-certificate "[WixBundleExecutePackageCacheFolder]\\gnx-qa-root.cer" --root-sha256 $(var.QaRootSha256) --publisher-certificate "[WixBundleExecutePackageCacheFolder]\\gnx-qa-publisher.cer" --publisher-sha256 $(var.QaPublisherSha256) --operation repair --format json',
        ),
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

    qa_package = exe_packages["PrepareQaTrust"]
    qa_payloads = {
        node.attrib.get("Id"): node
        for node in qa_package
        if local_name(node) == "Payload"
    }
    expected_qa_payloads = {
        "QaRootCertificatePayload": ("$(var.QaRootCertificate)", "gnx-qa-root.cer"),
        "QaPublisherCertificatePayload": (
            "$(var.QaPublisherCertificate)",
            "gnx-qa-publisher.cer",
        ),
    }
    if set(qa_payloads) != set(expected_qa_payloads):
        fail(f"QA trust payload inventory differs: {sorted(qa_payloads)!r}")
    for payload_id, (source, name) in expected_qa_payloads.items():
        payload = qa_payloads[payload_id]
        if (
            payload.attrib.get("SourceFile") != source
            or payload.attrib.get("Name") != name
            or payload.attrib.get("Compressed") != "yes"
        ):
            fail(f"QA trust payload differs for {payload_id}")
    bundle_text = (INSTALLER / "source" / "bundle.wxs").read_text(encoding="utf-8")
    for marker in (
        "<?if $(var.QaTrustEnabled) = 1 ?>",
        "<?endif?>",
        "Production preprocessing removes this",
    ):
        if marker not in bundle_text:
            fail(f"QA-only Bundle preprocessing contract omits {marker!r}")

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
    arp_system_component = next(
        (
            node
            for node in product.iter()
            if local_name(node) == "Property"
            and node.attrib.get("Id") == "ARPSYSTEMCOMPONENT"
        ),
        None,
    )
    if arp_system_component is None or arp_system_component.attrib.get("Value") != "1":
        fail("internal MSI must independently suppress its Programs and Features entry")
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
        or tray_launch.attrib.get("Return") != "ignore"
        or "Directory" in tray_launch.attrib
    ):
        fail("tray launch must be detached and non-vital to installation")
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
    signing = (INSTALLER / "modules" / "signing.ps1").read_text(encoding="utf-8")
    msi_validation = (INSTALLER / "modules" / "msi.ps1").read_text(
        encoding="utf-8"
    )
    bundle_validation = (INSTALLER / "modules" / "bundle.ps1").read_text(
        encoding="utf-8"
    )
    for marker in (
        "release\\manifest.toml",
        "source\\product.wxs",
        "source\\bundle.wxs",
        "extensions\\deterministic-bundle",
        "Test-MaintenanceContract",
        "Test-MsiPayloadCoherence",
        "Test-InstalledMsiIdentity",
        "Invoke-AuthenticodeSign",
        "Invoke-BurnAuthenticodeSign",
        "SigningCertificateThumbprint",
        "QaSigning",
        "Release and QA signing reject self-signed publisher certificates",
        "create-qa-signing-certificate.ps1",
        "QaTrustEnabled",
        "http://timestamp.digicert.com",
        "SetLastWriteTimeUtc",
        "runtime-payload",
    ):
        if marker not in build:
            fail(f"release entry point omits {marker!r}")
    for artifact_name in (
        "gnx-bootstrap",
        "gnx-service",
        "gnx",
        "gnx-tray",
        "service-wrapper",
    ):
        if f"Name = '{artifact_name}'" not in build:
            fail(f"first-party signature inventory omits {artifact_name!r}")
    for marker in (
        "Test-CodeSigningCertificateTrust",
        "RequireAuthRoot $true",
        "Test-ReleaseArtifactSet",
        "ExpectedVersion",
        "Release artifact signature inventory must not be empty",
        "Smart App Control release signing requires an RSA certificate",
        "Windows AuthRoot store",
        "Test-QaCodeSigningCertificateTrust",
        "GNX Labs QA Root",
        "GNX Labs QA Publisher",
    ):
        if marker not in signing and marker not in build:
            fail(f"release signature coverage omits {marker!r}")
    for marker in (
        "installed-gnx-service",
        "installed-service-wrapper",
        "installed-gnx",
        "installed-gnx-tray",
        "Test-ReleaseArtifactSet",
    ):
        if marker not in msi_validation:
            fail(f"MSI signature/version coverage omits {marker!r}")
    for marker in (
        "gnx-bootstrap-install-podman.exe",
        "gnx-bootstrap-install-wsl.exe",
        "gnx-bootstrap-prepare-qa-trust.exe",
        "gnx-bootstrap-prepare.exe",
        "gnx-bootstrap-validate.exe",
        "Production Bundle must not contain QA trust certificates",
        "wixstdba.exe",
        "Test-TrustedAuthenticodeArtifact",
        "Test-ReleaseArtifactSet",
    ):
        if marker not in bundle_validation:
            fail(f"Burn signature/version coverage omits {marker!r}")
    if build.find("-Path $productMsi") > build.find("Test-InstalledMsiIdentity"):
        fail("installed MSI collision check must inspect the final signed package bytes")
    qa_certificate = (
        INSTALLER / "create-qa-signing-certificate.ps1"
    ).read_text(encoding="utf-8")
    for marker in (
        "CN=GNX Labs QA Root",
        "CN=GNX Labs QA Publisher",
        "AddYears(10)",
        "AddYears(2)",
        "pathlength=0",
        "1.3.6.1.5.5.7.3.3",
        "KeyExportPolicy NonExportable",
        "Cert:\\CurrentUser\\My",
        "StoreLocation]::CurrentUser",
        "gnx-qa-root.cer",
        "gnx-qa-publisher.cer",
        "HasPrivateKey",
        "Purpose = 'QaOnly'",
        "Exportable = $false",
    ):
        if marker not in qa_certificate:
            fail(f"QA certificate contract omits {marker!r}")
    lifecycle = (ROOT / "tools" / "qa-lifecycle.ps1").read_text(encoding="utf-8")
    for marker in (
        "Assert-Administrator",
        "Get-AuthenticodeSignature",
        "CN=GNX Labs QA Publisher",
        "Invoke-SetupOperation -Stage 'repair' -Action '/repair'",
        "Invoke-SetupOperation -Stage 'uninstall' -Action '/uninstall'",
        "Invoke-SetupOperation -Stage 'fresh-install' -Action '/install'",
        "Wait-Ready",
        "Expected one visible Setup and one hidden MSI registration",
        "Expected one tray process after fresh install",
    ):
        if marker not in lifecycle:
            fail(f"QA lifecycle contract omits {marker!r}")
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
    bootstrap_main = (
        ROOT / "apps" / "gnx-bootstrap" / "src" / "main.rs"
    ).read_text(encoding="utf-8")
    qa_trust = (
        ROOT / "apps" / "gnx-bootstrap" / "src" / "qa_trust.rs"
    ).read_text(encoding="utf-8")
    for marker in (
        "prepare-qa-trust",
        "--root-certificate",
        "--root-sha256",
        "--publisher-certificate",
        "--publisher-sha256",
        "qa_certificate_trust",
    ):
        if marker not in bootstrap_main:
            fail(f"native QA trust command omits {marker!r}")
    for marker in (
        "CERT_SYSTEM_STORE_LOCAL_MACHINE",
        'add_to_machine_store("Root"',
        'add_to_machine_store("TrustedPublisher"',
        "CERT_STORE_ADD_REPLACE_EXISTING",
        "MAX_CERTIFICATE_BYTES",
        "QA {name} certificate SHA-256 mismatch",
    ):
        if marker not in qa_trust:
            fail(f"native QA trust implementation omits {marker!r}")
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
    for marker in (
        "CreateEventW",
        "WaitForSingleObject",
        "OpenEventW",
        "SetEvent",
        "ConvertStringSecurityDescriptorToSecurityDescriptorW",
        "ERROR_ALREADY_EXISTS",
        "SHUTDOWN_REQUESTED",
    ):
        if marker not in service_shutdown:
            fail(f"bounded service shutdown omits {marker!r}")
    if "process::exit" in service_shutdown:
        fail("service shutdown still terminates the process abruptly")
    pipe = (
        ROOT
        / "apps"
        / "gnx-service"
        / "src"
        / "infrastructure"
        / "windows_pipe.rs"
    ).read_text(encoding="utf-8")
    for marker in (
        "PIPE_INSTANCES: usize = 4",
        "CLIENT_IO_TIMEOUT",
        "FILE_FLAG_OVERLAPPED",
        "GetOverlappedResultEx",
        "CancelIoEx",
    ):
        if marker not in pipe:
            fail(f"bounded local IPC omits {marker!r}")
    staging = (
        ROOT
        / "apps"
        / "gnx-bootstrap"
        / "src"
        / "dependencies"
        / "staging.rs"
    ).read_text(encoding="utf-8")
    staging_security = (
        ROOT
        / "apps"
        / "gnx-bootstrap"
        / "src"
        / "windows"
        / "security.rs"
    ).read_text(encoding="utf-8")
    for marker in (
        "secure_owned_tree",
        "open_validated_locked",
        "share_mode(FILE_SHARE_READ)",
        "FILE_FLAG_OPEN_REPARSE_POINT",
    ):
        if marker not in staging:
            fail(f"privileged dependency staging omits {marker!r}")
    for marker in (
        "HKEY_LOCAL_MACHINE",
        "Common AppData",
        "PROTECTED_DACL_SECURITY_INFORMATION",
        "FILE_ATTRIBUTE_REPARSE_POINT",
        "WINDOWS_SERVICE_SID",
    ):
        if marker not in staging_security:
            fail(f"protected installer root omits {marker!r}")
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
    if product_package.attrib.get("Manufacturer") != "GNX Labs":
        fail("MSI manufacturer must identify GNX Labs")
    if burn_bundle.attrib.get("Manufacturer") != "GNX Labs":
        fail("Burn manufacturer must identify GNX Labs")
    if (
        burn_bundle.attrib.get("Copyright")
        != "Copyright (c) 2008-2020 GNX Labs, Hector AB and other contributors"
    ):
        fail("Burn copyright must credit GNX Labs and Hector AB")
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
        != "https://github.com/mayas-alas/quetzalcoatl/blob/main/LICENSE"
    ):
        fail("Burn does not expose the canonical repository LICENSE")
    eula_links = [
        node
        for node in theme.iter()
        if local_name(node) == "Hypertext"
        and node.attrib.get("Name") == "EulaHyperlink"
    ]
    if len(eula_links) != 1:
        fail("Burn must expose exactly one functional legal hyperlink control")
    legal_text = eula_links[0].text or ""
    for legal_link in ("License Agreement", "Privacy Policy"):
        if legal_link not in legal_text:
            fail(f"Burn initial page omits the {legal_link} link")
    if legal_text.count('<a href="#">') != 2:
        fail("Burn legal control must expose two links to the canonical LICENSE")
    accept_boxes = [
        node
        for node in theme.iter()
        if local_name(node) == "Checkbox"
        and node.attrib.get("Name") == "EulaAcceptCheckbox"
    ]
    if (
        len(accept_boxes) != 1
        or "License Agreement and Privacy Policy" not in (accept_boxes[0].text or "")
    ):
        fail("Burn acceptance text must cover both legal links")
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
