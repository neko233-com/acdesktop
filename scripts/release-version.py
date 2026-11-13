#!/usr/bin/env python3
import re
import sys
from pathlib import Path


def release_version(text):
    section = re.search(r"(?ms)^\[workspace\.package\][ \t]*\r?$\n(.*?)(?=^\[|\Z)", text)
    match = re.search(r'^version[ \t]*=[ \t]*"([^"\r\n]+)"[ \t]*\r?$', section.group(1), re.M) if section else None
    version = match.group(1) if match else ""
    if not re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?", version):
        raise ValueError("Cargo.toml must contain a valid workspace.package.version")
    return version


if __name__ == "__main__":
    text = sys.stdin.read() if sys.argv[1:] == ["-"] else Path("Cargo.toml").read_text(encoding="utf-8")
    try:
        print(release_version(text))
    except ValueError as error:
        print(error, file=sys.stderr)
        sys.exit(1)
