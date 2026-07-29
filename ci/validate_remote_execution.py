#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

from shell_syntax import find_posix_shell, validate_shell_syntax

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "crates" / "gnx-service" / "src" / "runtime"
AGENT = ROOT / "runtime" / "payload" / "bin" / "gnx-runtime-agent"
EXPECTED = {
    "Ping": ("ping",),
    "PveClusterPrepare": ("pve-cluster-create", "prepare"),
    "PveClusterVerifyNode": ("pve-cluster-create", "verify-node"),
    "PveClusterCreate": ("pve-cluster-create", "create"),
    "PveClusterVerify": ("pve-cluster-create", "verify"),
    "PveClusterJoin": ("pve-cluster-create", "join"),
    "PveClusterConfirmMember": ("pve-cluster-create", "confirm-member"),
    "PveConfigure": ("pve-configure",),
    "TailscalePrepare": ("tailscale-prepare",),
    "TailscaleRename": ("tailscale-rename",),
}


def fail(message: str) -> None:
    print(f"remote-execution-validation: ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def validate_typed_operations() -> None:
    operation = (RUNTIME / "remote" / "operation.rs").read_text(encoding="utf-8")
    for variant, argv in EXPECTED.items():
        if variant not in operation:
            fail(f"typed runtime operation is absent: {variant}")
        literal = "&[" + ", ".join(f'"{part}"' for part in argv) + "]"
        if literal not in operation:
            fail(f"typed argv mapping differs for {variant}")

    client = (RUNTIME / "remote" / "client.rs").read_text(encoding="utf-8")
    if "operation: RuntimeOperation" not in client:
        fail("runtime agent client still accepts free-form arguments")
    all_source = "\n".join(path.read_text(encoding="utf-8") for path in RUNTIME.rglob("*.rs"))
    if re.search(r"runtime_agent(?:_output)?\s*\(\s*[^,]+,\s*\[", all_source):
        fail("runtime agent call site constructs a free-form argv array")



def explicit_remote_argv_arrays(source: str) -> list[str]:
    """Return explicit array literals used as the argv argument of machine transport calls.

    This is intentionally narrow: variable command builders are validated through their typed
    contracts, while direct array call sites must remain shell-free.
    """
    arrays: list[str] = []
    call = re.compile(r"machine_stdin(?:_output)?\s*\(")
    for match in call.finditer(source):
        cursor = match.end()
        depth = 1
        comma_count = 0
        in_string = False
        escaped = False
        array_start: int | None = None
        array_depth = 0
        index = cursor
        while index < len(source) and depth > 0:
            char = source[index]
            if in_string:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    in_string = False
                index += 1
                continue
            if char == '"':
                in_string = True
            elif char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
            elif depth == 1 and char == ",":
                comma_count += 1
            elif depth == 1 and comma_count == 1 and char == "[":
                array_start = index
                array_depth = 1
                index += 1
                while index < len(source) and array_depth > 0:
                    inner = source[index]
                    if inner == '"':
                        index += 1
                        while index < len(source):
                            if source[index] == "\\":
                                index += 2
                                continue
                            if source[index] == '"':
                                break
                            index += 1
                    elif inner == "[":
                        array_depth += 1
                    elif inner == "]":
                        array_depth -= 1
                    index += 1
                if array_start is not None and array_depth == 0:
                    arrays.append(source[array_start:index])
                break
            index += 1
    return arrays


def validate_direct_remote_argv(rust_source: str) -> None:
    shell_control = re.compile(r"(?:<|>|\||&&|\|\||\$\(|`)")
    for argv in explicit_remote_argv_arrays(rust_source):
        literals = re.findall(r'"((?:\\.|[^"\\])*)"', argv)
        for literal in literals:
            if shell_control.search(literal):
                fail(
                    "direct remote argv contains shell-control syntax; "
                    f"move variable data to stdin instead: {literal!r}"
                )

def validate_transport() -> None:
    transport = (RUNTIME / "remote" / "transport.rs").read_text(encoding="utf-8")
    limits = (RUNTIME / "remote" / "limits.rs").read_text(encoding="utf-8")
    for marker in (
        "MAX_REMOTE_INPUT_BYTES",
        "MAX_REMOTE_OUTPUT_BYTES",
        "REMOTE_COMMAND_TIMEOUT",
        "child.try_wait()",
        "child.kill()",
    ):
        if marker not in transport and marker not in limits:
            fail(f"remote transport guard is absent: {marker}")

    rust_source = "\n".join(path.read_text(encoding="utf-8") for path in RUNTIME.rglob("*.rs"))
    payload_source = "\n".join(
        path.read_text(encoding="utf-8")
        for path in (ROOT / "runtime" / "payload" / "bin").iterdir()
        if path.is_file()
    )
    combined = rust_source + "\n" + payload_source
    forbidden_fragments = (
        " sh -c ",
        " sh -eu -c ",
        " bash -c ",
    )
    for fragment in forbidden_fragments:
        if fragment in combined:
            fail(f"forbidden shell-string execution remains: {fragment!r}")

    forbidden_argv = re.compile(r'"(?:ba)?sh"\s*,\s*"-c"')
    if forbidden_argv.search(combined):
        fail("forbidden shell-string argv remains: sh/bash followed by -c")

    validate_direct_remote_argv(rust_source)


def validate_agent() -> None:
    source = AGENT.read_text(encoding="utf-8")
    for value in ("set -eu", "umask 077", "fail_usage", "[ \"$#\" -eq 1 ] || fail_usage"):
        if value not in source:
            fail(f"runtime agent hardening marker is absent: {value}")
    forbidden = ("nc -l", "ncat -l", "socat", "listen(", "0.0.0.0", "exec \"$@\"")
    if any(fragment in source for fragment in forbidden):
        fail("runtime agent exposes a listener or arbitrary execution")
    shell = find_posix_shell()
    if shell is None:
        print(
            "remote-execution-validation: WARNING: runtime agent shell syntax skipped; "
            "install Git Bash or set GNX_SH to a POSIX sh executable",
            file=sys.stderr,
        )
        return

    error = validate_shell_syntax(AGENT, shell)
    if error is not None:
        fail(f"runtime agent shell syntax failed: {error}")


def main() -> None:
    validate_typed_operations()
    validate_transport()
    validate_agent()
    print("remote-execution-validation: ok")


if __name__ == "__main__":
    main()
