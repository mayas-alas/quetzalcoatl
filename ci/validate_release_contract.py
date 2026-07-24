#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.1.12"
PRODUCT = "{621E6E65-5BB1-5495-A887-F3AF3AA57125}"
PACKAGE = "{329DE363-62FE-5FBA-A820-0F90A9630411}"
BUNDLE = "{42D4F602-1355-5B82-B60C-2E5D7F03BFB5}"
PREVIOUS_PRODUCT = "{D0A35E80-8D6D-5C16-9C72-E233A92858DB}"
PREVIOUS_PACKAGE = "{82CE2E46-63AB-5475-B4C6-ABC5C469C964}"
PREVIOUS_BUNDLE = "{11F52020-5187-5E79-B5C3-434CF943E61D}"
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
    if package.attrib.get("ProductCode") != PRODUCT or package.attrib.get("UpgradeCode") != UPGRADE:
        fail("MSI release identity differs")
    if bundle.attrib.get("ProviderKey") != BUNDLE_UPGRADE or bundle.attrib.get("UpgradeCode") != BUNDLE_UPGRADE:
        fail("Burn upgrade family differs")
    bundle_source = (ROOT / "installer" / "bundle.wxs").read_text(encoding="utf-8")
    for cache in (f"gnx-host-prepare-{VERSION}", f"gnx-host-validate-{VERSION}"):
        if cache not in bundle_source:
            fail(f"Burn cache identity differs: {cache}")

    extension = (ROOT / "installer" / "wixext" / "Gnx.DeterministicBundle.wixext" / "DeterministicBundleExtension.cs").read_text(encoding="utf-8")
    if BUNDLE not in extension:
        fail("deterministic Burn extension uses a different BundleId")
    if len({PRODUCT, PACKAGE, BUNDLE, PREVIOUS_PRODUCT, PREVIOUS_PACKAGE, PREVIOUS_BUNDLE}) != 6:
        fail("release identity was reused")

    print("release-contract-validation: ok")


if __name__ == "__main__":
    main()
