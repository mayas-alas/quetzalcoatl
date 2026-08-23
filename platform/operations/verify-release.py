#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import hmac
import json
import re
import sys
from pathlib import Path

KEY_PATH = Path("/var/lib/quetzalcoatl/platform/secrets/release-signing-key")
MAX_RECORD_BYTES = 4096


def main() -> None:
    encoded = sys.stdin.buffer.read(MAX_RECORD_BYTES + 1)
    if not encoded or len(encoded) > MAX_RECORD_BYTES:
        raise SystemExit("release record size is invalid")
    record = json.loads(encoded)
    expected = {
        "schema",
        "source",
        "commit",
        "image",
        "port",
        "health_path",
        "service_slug",
        "signature",
    }
    if not isinstance(record, dict) or set(record) != expected:
        raise SystemExit("release record schema differs")
    signature = record.pop("signature")
    payload = json.dumps(
        record, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")
    key = bytes.fromhex(KEY_PATH.read_text(encoding="ascii").strip())
    expected_signature = hmac.new(key, payload, hashlib.sha256).hexdigest()
    if (
        len(key) != 32
        or not isinstance(signature, str)
        or not hmac.compare_digest(signature, expected_signature)
    ):
        raise SystemExit("release record authentication failed")
    slug = str(record["service_slug"])
    source = str(record["source"])
    image = str(record["image"])
    health_path = str(record["health_path"])
    if (
        record["schema"] != 2
        or not re.fullmatch(r"[a-z0-9][a-z0-9-]{0,31}", slug)
        or not re.fullmatch(r"gnx-labs/[a-z0-9][a-z0-9._-]{0,63}", source)
        or not re.fullmatch(
            rf"gnx-forgejo\.[a-z0-9.-]+\.ts\.net/{re.escape(source)}@sha256:[0-9a-f]{{64}}",
            image,
        )
        or not isinstance(record["port"], int)
        or not 1 <= record["port"] <= 65535
        or not re.fullmatch(r"/[a-z0-9._/-]{0,127}", health_path)
        or not re.fullmatch(r"[0-9a-f]{40}", str(record["commit"]))
    ):
        raise SystemExit("release record values differ")
    print(
        "|".join(
            (
                slug,
                source,
                image,
                str(record["port"]),
                health_path,
            )
        )
    )


if __name__ == "__main__":
    main()