#!/usr/bin/env python3
from __future__ import annotations

import base64
import hashlib
import hmac
import json
import os
import re
import ssl
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

STATE_ROOT = Path("/var/lib/quetzalcoatl/platform")
SECRET_ROOT = STATE_ROOT / "secrets"
RELEASE_ROOT = STATE_ROOT / "releases"
TAILNET = os.environ.get("GNX_TAILNET", "")
API_TOKEN = (SECRET_ROOT / "forgejo-api-token").read_text(encoding="utf-8").strip()
REGISTRY_TOKEN = (SECRET_ROOT / "forgejo-registry-token").read_text(
    encoding="utf-8"
).strip()
SIGNING_KEY = bytes.fromhex(
    (SECRET_ROOT / "release-signing-key").read_text(encoding="ascii").strip()
)

if not re.fullmatch(r"[a-z0-9](?:[a-z0-9.-]{1,61}[a-z0-9])?\.ts\.net", TAILNET):
    raise SystemExit("release discovery received an invalid tailnet")
if len(API_TOKEN) < 20 or len(REGISTRY_TOKEN) < 20 or len(SIGNING_KEY) != 32:
    raise SystemExit("release discovery secret contract differs")

FORGEJO = f"https://gnx-forgejo.{TAILNET}"
REGISTRY = f"gnx-forgejo.{TAILNET}"
SOURCE_PATTERN = re.compile(r"gnx-labs/[a-z0-9][a-z0-9._-]{0,63}")
DIGEST_PATTERN = re.compile(r"sha256:[0-9a-f]{64}")


def api(path: str) -> object:
    request = urllib.request.Request(
        f"{FORGEJO}{path}",
        headers={
            "Accept": "application/json",
            "Authorization": f"token {API_TOKEN}",
        },
    )
    with urllib.request.urlopen(request, timeout=20, context=ssl.create_default_context()) as response:
        if response.status != 200:
            raise RuntimeError(f"Forgejo API returned HTTP {response.status}")
        return json.load(response)


def registry_has_manifest(repository: str, digest: str) -> bool:
    credentials = base64.b64encode(f"gnx-admin:{REGISTRY_TOKEN}".encode()).decode()
    request = urllib.request.Request(
        f"https://{REGISTRY}/v2/{repository}/manifests/{digest}",
        method="HEAD",
        headers={
            "Accept": (
                "application/vnd.oci.image.manifest.v1+json,"
                "application/vnd.docker.distribution.manifest.v2+json"
            ),
            "Authorization": f"Basic {credentials}",
        },
    )
    try:
        with urllib.request.urlopen(
            request, timeout=20, context=ssl.create_default_context()
        ) as response:
            return response.status == 200
    except urllib.error.HTTPError as error:
        if error.code in (401, 404):
            return False
        raise


def canonical_payload(record: dict[str, object]) -> bytes:
    unsigned = {key: value for key, value in record.items() if key != "signature"}
    return json.dumps(
        unsigned, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")


def valid_signature(record: dict[str, object]) -> bool:
    signature = record.get("signature")
    return isinstance(signature, str) and hmac.compare_digest(
        signature, hmac.new(SIGNING_KEY, canonical_payload(record), hashlib.sha256).hexdigest()
    )


def service_identity(source: str) -> str:
    repository = source.split("/", 1)[1]
    normalized = re.sub(r"[^a-z0-9]+", "-", repository.lower()).strip("-")
    return normalized[:31].rstrip("-")


def load_declaration(repository: dict[str, object]) -> dict[str, object] | None:
    full_name = repository.get("full_name")
    if not isinstance(full_name, str) or not SOURCE_PATTERN.fullmatch(full_name):
        return None
    if full_name == "gnx-labs/gnx-service-template":
        return None
    quoted = urllib.parse.quote(full_name, safe="/")
    try:
        content = api(f"/api/v1/repos/{quoted}/contents/gnx.release.json?ref=main")
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return None
        raise
    if not isinstance(content, dict) or not isinstance(content.get("content"), str):
        raise RuntimeError(f"{full_name} returned an invalid release object")
    try:
        declaration = json.loads(base64.b64decode(content["content"], validate=True))
    except (ValueError, json.JSONDecodeError) as error:
        raise RuntimeError(f"{full_name} release declaration is invalid") from error
    expected_keys = {
        "schema",
        "source",
        "commit",
        "image",
        "port",
        "health_path",
    }
    if not isinstance(declaration, dict) or set(declaration) != expected_keys:
        raise RuntimeError(f"{full_name} release schema differs")
    if (
        declaration["schema"] != 2
        or declaration["source"] != full_name
        or not re.fullmatch(r"[0-9a-f]{40}", str(declaration["commit"]))
        or not isinstance(declaration["port"], int)
        or not 1 <= declaration["port"] <= 65535
        or not isinstance(declaration["health_path"], str)
        or not re.fullmatch(r"/[a-z0-9._/-]{0,127}", declaration["health_path"])
    ):
        raise RuntimeError(f"{full_name} release values differ from the fixed contract")
    image = str(declaration["image"])
    prefix = f"{REGISTRY}/{full_name}@"
    if not image.startswith(prefix) or not DIGEST_PATTERN.fullmatch(image[len(prefix) :]):
        raise RuntimeError(f"{full_name} image is not an immutable sovereign digest")
    if not registry_has_manifest(full_name, image[len(prefix) :]):
        raise RuntimeError(f"{full_name} declared OCI manifest is unavailable")
    return declaration


def write_record(declaration: dict[str, object]) -> Path:
    source = str(declaration["source"])
    slug = service_identity(source)
    target = RELEASE_ROOT / f"{slug}.json"
    if target.exists():
        previous = json.loads(target.read_text(encoding="utf-8"))
        if not isinstance(previous, dict) or not valid_signature(previous):
            raise RuntimeError(f"persisted release record for {source} failed authentication")
        if (
            previous.get("source") != source
            or previous.get("service_slug") != slug
        ):
            raise RuntimeError(f"persisted service identity for {source} drifted")
    record: dict[str, object] = {
        **declaration,
        "service_slug": slug,
    }
    record["signature"] = hmac.new(
        SIGNING_KEY, canonical_payload(record), hashlib.sha256
    ).hexdigest()
    encoded = json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n"
    temporary = target.with_suffix(".json.gnx-new")
    descriptor = os.open(
        temporary, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600
    )
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(encoded)
            stream.flush()
            os.fsync(stream.fileno())
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise
    os.replace(temporary, target)
    return target


def main() -> None:
    RELEASE_ROOT.mkdir(mode=0o700, parents=True, exist_ok=True)
    repositories = api("/api/v1/orgs/gnx-labs/repos?limit=50")
    if not isinstance(repositories, list) or len(repositories) > 50:
        raise RuntimeError("Forgejo repository inventory is not bounded")
    discovered: list[Path] = []
    for repository in repositories:
        if not isinstance(repository, dict):
            raise RuntimeError("Forgejo repository inventory is invalid")
        declaration = load_declaration(repository)
        if declaration is not None:
            discovered.append(write_record(declaration))
    for path in sorted(discovered):
        print(path)


if __name__ == "__main__":
    main()
