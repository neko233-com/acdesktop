from pathlib import Path
import json
import sys

TARGETS = (
    "aarch64-apple-darwin", "x86_64-apple-darwin",
    "aarch64-unknown-linux-gnu", "x86_64-unknown-linux-gnu",
    "aarch64-pc-windows-msvc", "x86_64-pc-windows-msvc",
)


def expected_assets(version):
    cli = {f"acdesktop-v{version}-{target}.tar.gz" for target in TARGETS}
    ffi = {f"acdesktop-ffi-v{version}-{target}.{'zip' if 'windows' in target else 'tar.gz'}" for target in TARGETS}
    return cli | ffi | {"checksums.txt", "install.sh", "install.ps1"}


def validate(version, names):
    names = list(names)
    expected = expected_assets(version)
    actual = set(names)
    return sorted(expected - actual), sorted(actual - expected), len(names) != len(actual)


def validate_checksums(version, text):
    names = []
    for line in text.splitlines():
        parts = line.split(None, 1)
        if len(parts) != 2 or len(parts[0]) != 64 or any(c not in "0123456789abcdefABCDEF" for c in parts[0]):
            return False
        names.append(parts[1].lstrip("*"))
    expected = expected_assets(version) - {"checksums.txt", "install.sh", "install.ps1"}
    return len(names) == len(set(names)) and set(names) == expected


def main():
    if len(sys.argv) != 3 or sys.argv[1] not in {"local", "remote"}:
        print("usage: check-release-assets.py local|remote VERSION", file=sys.stderr)
        return 2
    version = sys.argv[2]
    if sys.argv[1] == "local":
        names = [p.name for p in Path.cwd().iterdir() if p.is_file() and (p.name.endswith((".tar.gz", ".zip")) or p.name in {"checksums.txt", "install.sh", "install.ps1"})]
    else:
        names = [asset["name"] for asset in json.load(sys.stdin)["assets"]]
    if sys.argv[1] == "local" and not validate_checksums(version, Path("checksums.txt").read_text(encoding="utf-8")):
        print("Checksum manifest does not name each archive exactly once", file=sys.stderr)
        return 1
    missing, extra, duplicate = validate(version, names)
    if missing or extra or duplicate:
        print(f"Invalid release assets: missing={missing}, extra={extra}, duplicate={duplicate}", file=sys.stderr)
        return 1
    print(f"OK: exact release asset set for v{version}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
