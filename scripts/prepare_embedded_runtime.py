#!/usr/bin/env python3
"""Stage and validate the embedded Unix runtime for a Tauri build."""

from __future__ import annotations

import argparse
import shutil
import tarfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
ARCHIVE_NAME = f"Moraxterm_EmbeddedUnix_x64_{VERSION}.tar.xz"
REQUIRED_MEMBERS = {"./usr/bin/fish.exe", "./usr/bin/bash.exe", "./usr/bin/ssh.exe", "./usr/bin/telnet.exe", "./usr/bin/dig"}


def stage(source: Path, destination: Path) -> None:
    if not source.is_file():
        raise SystemExit(f"embedded runtime archive not found: {source}")
    with tarfile.open(source, "r:xz") as archive:
        names = {member.name for member in archive.getmembers()}
    missing = REQUIRED_MEMBERS - names
    if missing:
        raise SystemExit("embedded runtime archive is missing: " + ", ".join(sorted(missing)))
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    print(f"staged {destination} ({destination.stat().st_size} bytes)")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--url")
    parser.add_argument("--destination", type=Path, default=Path("tauri-ui/src-tauri/resources") / ARCHIVE_NAME)
    args = parser.parse_args()
    if bool(args.archive) == bool(args.url):
        parser.error("provide exactly one of --archive or --url")
    if args.url:
        temporary = args.destination.with_suffix(args.destination.suffix + ".download")
        temporary.parent.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(args.url, temporary)
        try:
            stage(temporary, args.destination)
        finally:
            temporary.unlink(missing_ok=True)
    else:
        stage(args.archive, args.destination)


if __name__ == "__main__":
    main()
