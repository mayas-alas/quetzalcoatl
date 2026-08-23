#!/usr/bin/env python3
from __future__ import annotations

import ast
import hashlib
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PLATFORM = ROOT / "platform"


def fail(message: str) -> None:
    print(f"platform-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    manifest = tomllib.loads((PLATFORM / "manifest.toml").read_text(encoding="utf-8"))
    lock = json.loads((PLATFORM / "platform.lock.json").read_text(encoding="utf-8"))
    policy = {
        "tailscale_tag": "tag:quetzalcoatl-service",
        "public_exposure": False,
        "repository_commands": False,
    }
    if (
        manifest.get("schema_version") != 1
        or manifest.get("bundle_contract") != 1
        or manifest.get("policy") != policy
    ):
        fail("platform manifest contract differs")
    if (
        lock.get("schema_version") != 1
        or lock.get("bundle_contract") != 1
        or lock.get("policy")
        != {
            "mutable_image_tags_allowed": False,
            "embedded_secrets_allowed": False,
            "repository_commands_allowed": False,
        }
    ):
        fail("platform lock contract or policy differs")

    locked: set[str] = set()
    for entry in lock.get("files", []):
        relative = entry.get("path", "")
        if (
            not relative
            or relative != relative.lower()
            or "\\" in relative
            or ".." in Path(relative).parts
            or relative in locked
            or entry.get("mode") not in {"0644", "0755"}
        ):
            fail(f"invalid platform file metadata: {relative!r}")
        source = PLATFORM / relative
        if not source.is_file():
            fail(f"locked platform file is absent: {relative}")
        if hashlib.sha256(source.read_bytes()).hexdigest() != entry.get("sha256"):
            fail(f"platform SHA-256 differs: {relative}")
        locked.add(relative)

    actual = {
        path.relative_to(PLATFORM).as_posix()
        for path in PLATFORM.rglob("*")
        if path.is_file() and path.name != "platform.lock.json"
    }
    if actual != locked:
        fail(
            "platform inventory differs: "
            f"missing={sorted(locked - actual)!r} unlocked={sorted(actual - locked)!r}"
        )

    expected_directories = {
        parent.as_posix()
        for relative in locked
        for parent in Path(relative).parents
        if parent != Path(".")
    }
    actual_directories = {
        path.relative_to(PLATFORM).as_posix()
        for path in PLATFORM.rglob("*")
        if path.is_dir()
    }
    if actual_directories != expected_directories:
        fail(
            "platform directory inventory differs: "
            f"missing={sorted(expected_directories - actual_directories)!r} "
            f"unlocked={sorted(actual_directories - expected_directories)!r}"
        )

    required = {
        "operations/deploy",
        "operations/discover-releases.py",
        "operations/forgejo-admin",
        "operations/lxc-host",
        "operations/lxc-service",
        "operations/reconcile",
        "operations/verify-release.py",
        "services/forgejo/compose.yml",
        "services/forgejo/serve.json",
        "services/garage/compose.yml",
        "services/garage/serve.json",
        "services/runner/compose.yml",
        "services/service/compose.yml",
        "services/service/serve.json",
        "services/freellmapi/compose.yml",
        "services/freellmapi/serve.json",
        "services/omniroute/compose.yml",
        "services/omniroute/serve.json",
        "tofu/foundation/entrypoint",
        "tofu/foundation/main.tf",
        "tofu/foundation/versions.tf",
        "tofu/service/entrypoint",
        "tofu/service/main.tf",
        "tofu/service/versions.tf",
    }
    if missing := required - actual:
        fail(f"platform omits runtime files: {sorted(missing)!r}")

    operations = "\n".join(
        (PLATFORM / "operations" / name).read_text(encoding="utf-8")
        for name in ("reconcile", "deploy", "forgejo-admin", "lxc-host", "lxc-service")
    )
    for marker in (
        "pct exec \"$vmid\" -- /bin/sh -s",
        'guest_script "$vmid" "$bundle/operations/lxc-host"',
        "LXC_HOST_FAILED=",
        '"authKey": "file:/run/secrets/authkey"',
        "--config=/run/gnx/tailscaled.json",
        'docker pull --quiet "$tailscale_image"',
        "--pull never",
        "network_mode: service:tailscale",
        "TF_REGISTRY_DISCOVERY_RETRY=5",
        "TF_PROVIDER_DOWNLOAD_RETRY=5",
        "TF_REGISTRY_CLIENT_TIMEOUT=20",
    ):
        source = operations
        if marker == "network_mode: service:tailscale":
            source = "\n".join(
                path.read_text(encoding="utf-8")
                for path in (PLATFORM / "services").rglob("*.yml")
            )
        if marker not in source:
            fail(f"platform omits closed bootstrap contract: {marker}")

    reconcile = (PLATFORM / "operations" / "reconcile").read_text(encoding="utf-8")
    deploy = (PLATFORM / "operations" / "deploy").read_text(encoding="utf-8")
    lxc_host = (PLATFORM / "operations" / "lxc-host").read_text(encoding="utf-8")
    forgejo_admin = (PLATFORM / "operations" / "forgejo-admin").read_text(
        encoding="utf-8"
    )
    for helper in ("guest_script", "guest_directory", "guest_put", "guest_input"):
        if f"{helper}() (" not in reconcile:
            fail(f"reconcile {helper} must isolate its variables from the caller")
        if f"{helper}() (" not in deploy:
            fail(f"deploy {helper} must isolate its variables from the caller")
    if "prepare_service_assets() (" not in reconcile:
        fail("service asset preparation must isolate its directory variable")
    for helper in ("ensure_hex", "ensure_access_key"):
        if f"{helper}() (" not in reconcile:
            fail(f"{helper} must isolate its variables from the caller")
    if "ensure_base64url_32() (" not in reconcile:
        fail("Forgejo JWT secret generation must isolate its variables")
    if "printf 'GK%s\\n'" in reconcile:
        fail("durable access keys must not contain protocol delimiters")
    if 'printf \'%s\\n\' "$value" > "$secret_root/$target.gnx-new"' in reconcile:
        fail("returned platform secrets must not contain protocol delimiters")
    lxc_service = (PLATFORM / "operations" / "lxc-service").read_text(
        encoding="utf-8"
    )
    if 'printf \'%s\\n\' "$forgejo_runner_secret"' in lxc_service:
        fail("Forgejo runner secret file must contain exactly 40 hexadecimal bytes")
    if 'printf \'%s\' "$forgejo_runner_secret"' not in lxc_service:
        fail("Forgejo runner secret file lacks its delimiter-free write contract")
    if (
        'printf \'FORGEJO_API_TOKEN=%s\\n\' "$platform_api_token"'
        not in lxc_service
        or 'printf \'FORGEJO_REGISTRY_TOKEN=%s\\n\' "$registry_reader_token"'
        not in lxc_service
        or "forgejo_status=$?" not in reconcile
        or '[ "$forgejo_status" -eq 0 ] || exit 71' not in reconcile
    ):
        fail("Forgejo bootstrap does not checkpoint generated tokens before failure")
    for token_contract in (
        'name.startswith(("gnx-platform-reconciler", "gnx-registry-reader"))',
        'request(f"/users/gnx-admin/tokens/{token_id}", "DELETE", accepted=(204,))',
        'print(create("gnx-platform-reconciler", ["all"]))',
        'print(create("gnx-registry-reader", ["read:package"]))',
        "normalize_optional_token_pair",
    ):
        if token_contract not in f"{lxc_service}\n{reconcile}":
            fail(f"Forgejo token reconciliation omits {token_contract!r}")
    for runner_contract in (
        "JWT_SECRET_URI = file:/run/secrets/forgejo/jwt_secret",
        'ensure_base64url_32 "$secret_root/forgejo-jwt-secret"',
        'docker exec "$container" ip link set dev tailscale0 mtu 1100',
        "constrain_tailscale_mtu gnx-garage-tailscale",
        "constrain_tailscale_mtu gnx-forgejo-tailscale",
        "constrain_tailscale_mtu gnx-runner-tailscale",
        'constrain_tailscale_mtu "gnx-service-$slug-tailscale"',
        "docker restart gnx-forgejo >/dev/null",
        'runner_uuid=$(printf \'%s\\n\' "$runner_output" | tail -n 1)',
        "grep -Eq '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'",
    ):
        if runner_contract not in f"{lxc_service}\n{reconcile}":
            fail(f"Forgejo runner reconciliation omits {runner_contract!r}")
    if "TS_AUTHKEY=" in operations:
        fail("Tailscale auth keys must use the transient declarative config")
    if "backend_override.tf.gnx-new" in operations or "-f backend_override.tf" in operations:
        fail("the generated backend declaration must not use override semantics")
    if 'terraform { backend "s3" {} }' in operations:
        fail("nested HCL blocks must not use single-line block syntax")
    for fail_closed_probe in (
        'docker-ce 2>/dev/null)" = "$docker_version" || return 1',
        "systemctl is-enabled --quiet docker.service || return 1",
        "systemctl is-active --quiet docker.service || return 1",
        "docker compose version --short | grep -Eq '^5\\.3\\.1$' || return 1",
    ):
        if fail_closed_probe not in lxc_host:
            fail(f"LXC host verification can fail open: {fail_closed_probe}")
    if "grep -c '^LXC_HOST=ready$' \"$host_log\"" not in reconcile:
        fail("reconcile does not require one LXC host completion marker")
    for credential_contract in (
        "FORGEJO_ADMIN_USERNAME=gnx-admin",
        "FORGEJO_ADMIN_PASSWORD=%s",
        'exec 9> "$state_root/operation.lock"',
        'flock -x 9',
        'method="PATCH"',
        '"password": password',
        'Authorization": f"Basic {authorization}',
    ):
        if credential_contract not in forgejo_admin:
            fail(f"Forgejo admin operation omits {credential_contract!r}")
    for forbidden_secret_transport in (
        "--password $password",
        'PASSWORD="$password"',
        "docker exec gnx-forgejo forgejo admin user change-password",
    ):
        if forbidden_secret_transport in forgejo_admin:
            fail("Forgejo admin password escaped into argv or environment")

    referenced = {
        match.group(1)
        for match in re.finditer(r'\$bundle/([a-z0-9_./-]+)', operations)
        if not match.group(1).endswith("/")
    }
    if missing := referenced - actual:
        fail(f"platform operation references absent files: {sorted(missing)!r}")

    for script in ("discover-releases.py", "verify-release.py"):
        try:
            ast.parse(
                (PLATFORM / "operations" / script).read_text(encoding="utf-8"),
                filename=script,
            )
        except SyntaxError as error:
            fail(f"platform Python operation is invalid: {error}")

    platform_source = "\n".join(
        path.read_text(encoding="utf-8")
        for path in PLATFORM.rglob("*")
        if path.is_file() and path.name != "platform.lock.json"
    )
    for forbidden in (
        "provisioner ",
        "ansible/",
        "ansible-playbook",
        "community.proxmox",
        "pct_remote",
        "sh -c",
        "bash -c",
        "--funnel",
        "tailscale funnel",
        ".github/workflows",
    ):
        if forbidden in platform_source:
            fail(f"platform contains forbidden surface: {forbidden}")
    if re.search(r"tskey-auth-[A-Za-z0-9-]{10,}", platform_source):
        fail("platform source contains an auth key")
    if re.search(r"image:\s+\S+:(?:latest|main|master)\b", platform_source):
        fail("platform source contains a mutable image tag")
    for mutable_action in (
        "actions/checkout@v",
        "docker/login-action@v",
        "docker/build-push-action@v",
    ):
        if mutable_action in platform_source:
            fail(f"template workflow uses a mutable action: {mutable_action}")

    print("platform-validation: ok")


if __name__ == "__main__":
    main()
