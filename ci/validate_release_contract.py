#!/usr/bin/env python3
from __future__ import annotations

import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.1.14"
PRODUCT = "{ACFA43DA-DDE5-501B-A773-C50BED15F59F}"
PACKAGE = "{ACE7E7A7-7411-5444-8DD3-3DBF7F2DCAD2}"
BUNDLE = "{C7F7AE72-0CA0-5D2E-96B4-E91C50C294B9}"
PREVIOUS_PRODUCT = "{56E3CF39-864C-51F8-BE28-86C9ADE58118}"
PREVIOUS_PACKAGE = "{96520581-4D5C-53CA-80F8-8329F919CA69}"
PREVIOUS_BUNDLE = "{8C9449BC-368E-516A-BEEF-CFA0D3C243E7}"
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
    bundle = ET.parse(ROOT / "installer" / "bundle.wxs").getroot().find("w:Bundle", ns)
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
    for cache in (f"gnx-host-prepare-{VERSION}", f"gnx-host-validate-{VERSION}"):
        if cache not in bundle_source:
            fail(f"Burn cache identity differs: {cache}")

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
