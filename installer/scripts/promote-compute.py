#!/usr/bin/env python3
"""Explicit OCI promotion via Skopeo, not a product HTTP download channel."""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def skopeo(*arguments):
    return subprocess.run(["skopeo", *arguments], check=True, capture_output=True).stdout


def validate(configuration, root=ROOT):
    config = json.loads(Path(configuration).read_text())
    if config.get("status") != "candidate":
        raise ValueError("Only an explicit candidate can be promoted")
    if not re.fullmatch(r"[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}", config["release"]):
        raise ValueError("Invalid release")
    if not re.fullmatch(r"sha256:[0-9a-f]{64}", config["manifest_digest"]):
        raise ValueError("Invalid manifest digest")
    if not re.fullmatch(r"[0-9a-f]{64}", config["archive_sha256"]):
        raise ValueError("Invalid archive hash")
    if config["archive_path"] != "build/compute/proxmox.oci.tar":
        raise ValueError("Candidate must use the canonical build output")
    repository = config.get("registry_repository")
    if repository is not None and not re.fullmatch(
        r"[a-z0-9][a-z0-9.-]+(?::[0-9]{1,5})?/[a-z0-9][a-z0-9_./-]*", repository
    ):
        raise ValueError("Invalid OCI registry repository")
    if repository is not None and (".." in repository or "//" in repository):
        raise ValueError("Invalid OCI registry repository")
    archive = root / config["archive_path"]
    with archive.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    if digest != config["archive_sha256"]:
        raise ValueError("Candidate archive hash mismatch; explicit review required")
    source = "oci-archive:" + str(archive)
    manifest = skopeo("inspect", "--raw", source)
    if "sha256:" + hashlib.sha256(manifest).hexdigest() != config["manifest_digest"]:
        raise ValueError("Candidate manifest mismatch")
    image = json.loads(skopeo("inspect", "--config", source))
    if image["architecture"] != "amd64" or image["os"] != "linux":
        raise ValueError("Product requires linux/amd64")
    if image["config"]["Entrypoint"] != ["/usr/local/bin/entrypoint.sh"]:
        raise ValueError("Unexpected compute entrypoint")
    if image["config"]["Cmd"] != ["/sbin/init", "--log-target=console", "--log-level=notice"]:
        raise ValueError("Unexpected compute init")
    if image["config"]["Labels"]["org.gnx.compute.release"] != config["release"]:
        raise ValueError("Candidate label mismatch")
    return config, source


def publish(config, source):
    repository = config.get("registry_repository")
    if not repository:
        raise ValueError("Set registry_repository to the authorized product OCI repository first")
    tag = repository + ":" + config["release"]
    reference = repository + "@" + config["manifest_digest"]
    # No credentials on argv or environment: use the standard operator-managed
    # containers auth store and Skopeo policy. Never disable TLS/signature policy.
    existing = subprocess.run(
        ["skopeo", "inspect", "--tls-verify=true", "--no-tags", "--raw", "docker://" + tag],
        capture_output=True,
    )
    if existing.returncode == 0:
        actual = "sha256:" + hashlib.sha256(existing.stdout).hexdigest()
        if actual != config["manifest_digest"]:
            raise ValueError("Immutable registry release tag already names another image")
    else:
        error = existing.stderr.decode(errors="replace").lower()
        if not any(marker in error for marker in ("manifest unknown", "name unknown", "statuscode: 404")):
            raise ValueError("Registry preflight failed; verify repository and protected auth store")
        skopeo("copy", "--preserve-digests", "--dest-tls-verify=true", source, "docker://" + tag)
    manifest = skopeo("inspect", "--tls-verify=true", "--no-tags", "--raw", "docker://" + reference)
    if "sha256:" + hashlib.sha256(manifest).hexdigest() != config["manifest_digest"]:
        raise ValueError("Remote manifest did not preserve the candidate digest")
    promoted = {
        "status": "promoted", "release": config["release"],
        "source": "docker://" + reference, "image": reference,
        "storage_tag": tag, "manifest_digest": config["manifest_digest"],
    }
    output = ROOT / "build/compute/promoted.json"
    output.write_text(json.dumps(promoted, indent=2) + "\n")
    print("OCI registry verified:", reference)
    print("Promotion record:", output)
    print("Review and package this record with the matching Quadlet; production is unchanged.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=["check", "publish"])
    parser.add_argument("--config", type=Path, default=ROOT / "payload/node/compute/image/release.json")
    arguments = parser.parse_args()
    config, source = validate(arguments.config)
    if arguments.operation == "publish":
        publish(config, source)
    else:
        print("Validated OCI candidate:", config["manifest_digest"])
        print("Registry:", config.get("registry_repository") or "NOT CONFIGURED — promotion blocked")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit("Compute promotion blocked: " + (str(error) if not isinstance(error, subprocess.CalledProcessError) else "Skopeo operation failed"))
