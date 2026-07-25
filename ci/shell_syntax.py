#!/usr/bin/env python3
from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path


def find_posix_shell() -> str | None:
    configured = os.environ.get("GNX_SH")
    if configured:
        path = Path(configured).expanduser()
        if path.is_file():
            return str(path)

    resolved = shutil.which("sh")
    if resolved:
        return resolved

    if os.name != "nt":
        return None

    roots = [
        os.environ.get("ProgramFiles"),
        os.environ.get("ProgramFiles(x86)"),
        os.environ.get("LocalAppData"),
        os.environ.get("ChocolateyInstall"),
        r"C:\msys64",
    ]
    suffixes = (
        Path("Git/bin/sh.exe"),
        Path("Git/usr/bin/sh.exe"),
        Path("Programs/Git/bin/sh.exe"),
        Path("Programs/Git/usr/bin/sh.exe"),
        Path("bin/sh.exe"),
        Path("usr/bin/sh.exe"),
    )
    for root in roots:
        if not root:
            continue
        base = Path(root)
        for suffix in suffixes:
            candidate = base / suffix
            if candidate.is_file():
                return str(candidate)
    return None


def validate_shell_syntax(path: Path, shell: str) -> str | None:
    result = subprocess.run(
        [shell, "-n", str(path)],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode == 0:
        return None
    return result.stderr.strip() or result.stdout.strip() or f"exit code {result.returncode}"
