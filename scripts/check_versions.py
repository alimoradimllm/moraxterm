#!/usr/bin/env python3
"""Validate that release-facing project versions agree."""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$")


def read_version_file() -> str:
    value = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
    if not VERSION_RE.fullmatch(value):
        raise ValueError(f"invalid VERSION value: {value!r}")
    return value


def cargo_version(path: Path) -> str:
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip().startswith("version") and "=" in line:
            return line.split("=", 1)[1].strip().strip('"')
    raise ValueError(f"version not found in {path}")


def versions() -> dict[str, str]:
    package = json.loads((ROOT / "tauri-ui" / "package.json").read_text(encoding="utf-8"))
    return {
        "VERSION": read_version_file(),
        "tauri-ui/package.json": package["version"],
        "tauri-ui/src-tauri/Cargo.toml": cargo_version(ROOT / "tauri-ui" / "src-tauri" / "Cargo.toml"),
        "pyproject.toml": cargo_version(ROOT / "pyproject.toml"),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--print-embedded-runtime", action="store_true")
    args = parser.parse_args()
    try:
        values = versions()
    except (OSError, KeyError, TypeError, ValueError) as error:
        print(f"version check failed: {error}", file=sys.stderr)
        return 1
    unique = set(values.values())
    if len(unique) != 1:
        print("version mismatch:", file=sys.stderr)
        for name, value in values.items():
            print(f"  {name}: {value}", file=sys.stderr)
        return 1
    version = next(iter(unique))
    if args.print_embedded_runtime:
        print(f"Moraxterm_EmbeddedUnix_x64_{version}.tar.xz")
    else:
        print(f"version consistency: {version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
