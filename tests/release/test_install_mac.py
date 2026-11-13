import hashlib
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipIf(os.name == "nt", "The installer targets macOS shell semantics")
class MacInstallerTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.stage = self.root / "stage"
        self.stage.mkdir()
        for name in ("acdesktop", "acdesktop-macos-helper"):
            (self.stage / name).write_text("fixture")
        archive = self.root / "archive.tar.gz"
        with tarfile.open(archive, "w:gz") as tar:
            for name in ("acdesktop", "acdesktop-macos-helper"):
                tar.add(self.stage / name, arcname=name)
        name = "acdesktop-v1.2.3-aarch64-apple-darwin.tar.gz"
        (self.root / "checksums.txt").write_text(f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {name}\\n")
        self.write_executable("uname", '#!/bin/sh\n[ "$1" = -s ] && echo Darwin || echo arm64\n')
        self.write_executable("curl", '''#!/usr/bin/env python3
import os, shutil, sys
args = sys.argv[1:]
if "-w" in args:
    if args[args.index("-w")+1] != "%{url_effective}": sys.exit(9)
    print("https://github.com/neko233-com/acdesktop/releases/tag/v1.2.3", end="")
    sys.exit(0)
out = args[args.index("-o")+1]
url = args[-1]
if url.endswith("checksums.txt"):
    shutil.copyfile(os.environ["FIXTURE_CHECKSUMS"], out)
elif os.environ.get("FAIL_ARCHIVE") == "1":
    sys.exit(22)
else:
    shutil.copyfile(os.environ["FIXTURE_ARCHIVE"], out)
''')
        self.write_executable("shasum", '#!/bin/sh\nshift 2\nsha256sum "$@"\n')

    def write_executable(self, name, content):
        path = self.bin / name
        path.write_text(content)
        path.chmod(0o755)

    def run_installer(self, fail=False):
        env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}", HOME=str(self.root),
                   ACDESKTOP_INSTALL_DIR=str(self.root / "installed"), FIXTURE_ARCHIVE=str(self.root / "archive.tar.gz"),
                   FIXTURE_CHECKSUMS=str(self.root / "checksums.txt"), FAIL_ARCHIVE="1" if fail else "0")
        return subprocess.run(["bash", str(ROOT / "scripts/install.sh")], env=env, capture_output=True, text=True)

    def test_latest_release_download_and_install(self):
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.root / "installed/acdesktop").is_file())
        self.assertTrue((self.root / "installed/acdesktop-macos-helper").is_file())

    def test_failed_archive_download_cannot_install(self):
        result = self.run_installer(fail=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "installed/acdesktop").exists())
