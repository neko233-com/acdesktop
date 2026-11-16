from pathlib import Path
import difflib
import subprocess
import sys
import tempfile


def compare_headers(expected, generated):
    return list(difflib.unified_diff(
        expected.splitlines(), generated.splitlines(),
        fromfile="committed header", tofile="generated header", lineterm="",
    ))


def main():
    root = Path(__file__).resolve().parents[1]
    header = root / "crates/ffi/include/acdesktop.h"
    with tempfile.TemporaryDirectory(prefix="acdesktop-header-") as directory:
        generated = Path(directory) / "acdesktop.h"
        subprocess.run([
            "cbindgen", "crates/ffi", "--config", "crates/ffi/cbindgen.toml",
            "--output", str(generated),
        ], cwd=root, check=True)
        differences = compare_headers(
            header.read_text(encoding="utf-8"), generated.read_text(encoding="utf-8")
        )
    if differences:
        print("\n".join(differences), file=sys.stderr)
        return 1
    print("OK: generated C header matches the committed header")
    return 0


if __name__ == "__main__":
    sys.exit(main())
