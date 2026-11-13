import io
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/check-linux-archive-contents.sh"
HELPER = "acdesktop-linux-clipboard-owner"


class ArchiveContentsTest(unittest.TestCase):
    def archive(self, name, members):
        path = Path(self.temp.name) / name
        with tarfile.open(path, "w:gz") as archive:
            for member, (content, mode) in members.items():
                data = content.encode()
                item = tarfile.TarInfo(member)
                item.size = len(data)
                item.mode = mode
                archive.addfile(item, io.BytesIO(data))
        return path

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.cli = {"acdesktop": ("cli", 0o755), HELPER: ("helper", 0o755)}
        readme = (ROOT / "scripts/ffi-archive-readme-linux.md").read_text(encoding="utf-8") + HELPER
        self.ffi = {"lib/libacdesktop_ffi.so": ("lib", 0o644), f"lib/{HELPER}": ("helper", 0o755),
                    "include/acdesktop.h": ("header", 0o644), "LICENSE": ("license", 0o644),
                    "README.md": (readme, 0o644), "SHA256SUMS": ("sums", 0o644)}

    def run_check(self, cli=None, ffi=None):
        cli_path = self.archive("cli.tar.gz", self.cli if cli is None else cli)
        ffi_members = self.ffi if ffi is None else ffi
        ffi_path = self.archive("ffi.tar.gz", {f"release/{name}": value for name, value in ffi_members.items()})
        return subprocess.run(["bash", str(SCRIPT), "all", str(cli_path), str(ffi_path)], capture_output=True, text=True)

    def test_complete_archives_pass(self):
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_missing_or_nonexecutable_helpers_fail(self):
        for members, key in ((self.cli, HELPER), (self.ffi, f"lib/{HELPER}")):
            for replacement in (None, ("helper", 0o644)):
                altered = members.copy()
                if replacement is None:
                    del altered[key]
                else:
                    altered[key] = replacement
                result = self.run_check(cli=altered if members is self.cli else None,
                                        ffi=altered if members is self.ffi else None)
                self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_extra_cli_file_and_missing_readme_contract_fail(self):
        altered = dict(self.cli, stray=("stray", 0o755))
        self.assertNotEqual(self.run_check(cli=altered).returncode, 0)
        altered = dict(self.ffi, **{"README.md": ("incomplete", 0o644)})
        self.assertNotEqual(self.run_check(ffi=altered).returncode, 0)
