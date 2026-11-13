from pathlib import Path
import re
import sys

TARGETS = {
    "aarch64-apple-darwin", "x86_64-apple-darwin",
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
        if len(targets) != len(TARGETS) or set(targets) != TARGETS:
            errors.append(f"{name}: target matrix is incomplete or duplicated")
        if "scripts/checkout-release-source.sh" not in section:
            errors.append(f"{name}: release source verification missing")
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
