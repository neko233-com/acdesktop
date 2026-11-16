from pathlib import Path
import re
import sys

TARGETS = {
    "aarch64-unknown-linux-gnu", "x86_64-unknown-linux-gnu",
    "aarch64-pc-windows-msvc", "x86_64-pc-windows-msvc",
}
GATES = {"check-release", "build", "build-ffi", "ffi-release-gates", "ffi-release-panic-guard", "linux-archive-contents"}


def job(text, name):
    match = re.search(rf"(?ms)^  {re.escape(name)}:\s*\n(.*?)(?=^  [a-z][\w-]*:\s*$|\Z)", text)
    return match.group(1) if match else ""


def violations(text):
    errors = []
    for name in ("build", "build-ffi"):
        section = job(text, name)
        targets = re.findall(r"(?m)^\s*- target: (\S+)\s*$", section)
        expected = TARGETS | {"i686-pc-windows-msvc"} if name == "build" else TARGETS
        if len(targets) != len(expected) or set(targets) != expected:
            errors.append(f"{name}: target matrix is incomplete or duplicated")
        if "scripts/checkout-release-source.sh" not in section:
            errors.append(f"{name}: release source verification missing")
    ffi = job(text, "build-ffi")
    for required in ("Build Windows control indicator helper for FFI consumers", 'release/acdesktop.exe" "$STAGE/lib/"', 'FFI archive is missing $helper'):
        if required not in ffi:
            errors.append(f"build-ffi: missing Windows indicator helper: {required}")
    gate = job(text, "ffi-release-gates")
    for required in ("runs-on: windows-latest", "scripts/check-ffi-header.py", "c_abi_passthrough"):
        if required not in gate:
            errors.append(f"ffi-release-gates: missing {required}")
    if re.search(r"(?m)^\s*runner: macos-", text) or re.search(r"(?m)^\s*runs-on: macos-", text):
        errors.append("release workflow must not run macOS jobs")
    publish = job(text, "publish-github")
    needs = re.search(r"(?m)^    needs: \[([^\]]+)\]", publish)
    if not needs or set(needs.group(1).replace(" ", "").split(",")) != GATES:
        errors.append("publish-github: required gates missing")
    for required in ("contents: write", "id-token: write", "attestations: write", "actions/attest-build-provenance@", "checksums.txt", "install.sh", "install.ps1"):
        if required not in publish:
            errors.append(f"publish-github: missing {required}")
    for required in ("sha256sum --check checksums.txt", "check-release-assets.py local", "check-release-assets.py remote"):
        if required not in publish:
            errors.append(f"publish-github: missing {required}")
    if "always()" in publish or "--clobber" not in publish:
        errors.append("publish-github: unsafe gate or non-recoverable asset upload")
    for required in ("--draft", "--json isDraft --jq .isDraft", "gh release edit \"$TAG_NAME\" --draft=false"):
        if required not in publish:
            errors.append(f"publish-github: missing draft safety check {required}")
    if publish.find("actions/attest-build-provenance@") > publish.find("gh release edit \"$TAG_NAME\" --draft=false"):
        errors.append("publish-github: draft is published before provenance")
    if not re.search(r"(?m)^permissions: \{\}\s*$", text):
        errors.append("workflow must deny permissions by default")
    return errors


def main():
    path = Path(__file__).resolve().parents[1] / ".github/workflows/release.yml"
    problems = violations(path.read_text(encoding="utf-8"))
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    print("OK: release matrix, gates and provenance policy")
    return 0


if __name__ == "__main__":
    sys.exit(main())
