#!/usr/bin/env python3
from __future__ import annotations

import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.1.15"
PRODUCT = "{F5F93BC3-1E26-5F41-943A-17465E358D91}"
PACKAGE = "{D937B18F-E797-59AD-AB9A-0C334610D3F1}"
BUNDLE = "{47B57628-E063-5738-BB41-F574C1164B09}"
PREVIOUS_PRODUCT = "{ACFA43DA-DDE5-501B-A773-C50BED15F59F}"
PREVIOUS_PACKAGE = "{ACE7E7A7-7411-5444-8DD3-3DBF7F2DCAD2}"
PREVIOUS_BUNDLE = "{C7F7AE72-0CA0-5D2E-96B4-E91C50C294B9}"
UPGRADE = "{47D5BD44-D061-407B-913B-47D17EC3BEA9}"
BUNDLE_UPGRADE = "{10B764B2-36AE-4911-A8C8-2F1A2A963769}"


def fail(message: str) -> None:
    print(f"release-contract-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def require_assignment(source: str, name: str, value: str) -> None:
    if f'${name} = "{value}"' not in source:
        fail(f"installer assignment differs: {name}")


def main() -> None:
    build = (ROOT / "installer" / "build.ps1").read_text(encoding="utf-8")
    for name, value in (
        ("releaseVersion", VERSION),
        ("releaseProductCode", PRODUCT),
        ("releasePackageCode", PACKAGE),
        ("releaseBundleId", BUNDLE),
        ("previousProductCode", PREVIOUS_PRODUCT),
        ("previousPackageCode", PREVIOUS_PACKAGE),
        ("previousBundleId", PREVIOUS_BUNDLE),
        ("releaseUpgradeCode", UPGRADE),
        ("bundleUpgradeCode", BUNDLE_UPGRADE),
    ):
        require_assignment(build, name, value)

    ns = {"w": "http://wixtoolset.org/schemas/v4/wxs"}
    package = ET.parse(ROOT / "installer" / "package.wxs").getroot().find("w:Package", ns)
    bundle_root = ET.parse(ROOT / "installer" / "bundle.wxs").getroot()
    bundle = bundle_root.find("w:Bundle", ns)
    if package is None or bundle is None:
        fail("WiX package or bundle is absent")
    if package.attrib.get("Version") != VERSION or package.attrib.get("ProductCode") != PRODUCT:
        fail("MSI release identity differs")
    if package.attrib.get("UpgradeCode") != UPGRADE:
        fail("MSI upgrade family differs")
    if bundle.attrib.get("Version") != VERSION:
        fail("Burn version differs")
    if bundle.attrib.get("ProviderKey") != BUNDLE_UPGRADE or bundle.attrib.get("UpgradeCode") != BUNDLE_UPGRADE:
        fail("Burn upgrade family differs")

    bundle_source = (ROOT / "installer" / "bundle.wxs").read_text(encoding="utf-8")
    for cache in (
        f"gnx-host-prepare-{VERSION}",
        f"gnx-host-install-wsl-{VERSION}",
        f"gnx-host-install-podman-{VERSION}",
        f"gnx-host-validate-{VERSION}",
    ):
        if cache not in bundle_source:
            fail(f"Burn cache identity differs: {cache}")

    chain = bundle.find("w:Chain", ns)
    if chain is None:
        fail("Burn chain is absent")
    if chain.findall("w:MsiPackage[@Id='Wsl']", ns) or chain.findall("w:MsiPackage[@Id='Podman']", ns):
        fail("dependencies still execute as direct Burn MSI packages")
    for package_id, payload_id in (("InstallWsl", "WslMsiPayload"), ("InstallPodman", "PodmanMsiPayload")):
        helpers = chain.findall(f"w:ExePackage[@Id='{package_id}']", ns)
        if len(helpers) != 1 or len(helpers[0].findall(f"w:Payload[@Id='{payload_id}']", ns)) != 1:
            fail(f"stable dependency helper differs: {package_id}")

    extension = (
        ROOT / "installer" / "wixext" / "Gnx.DeterministicBundle.wixext" / "DeterministicBundleExtension.cs"
    ).read_text(encoding="utf-8")
    if BUNDLE not in extension:
        fail("deterministic Burn extension uses a different BundleId")
    if len({PRODUCT, PACKAGE, BUNDLE, PREVIOUS_PRODUCT, PREVIOUS_PACKAGE, PREVIOUS_BUNDLE}) != 6:
        fail("release identity was reused")

    print("release-contract-validation: ok")


if __name__ == "__main__":
    main()
