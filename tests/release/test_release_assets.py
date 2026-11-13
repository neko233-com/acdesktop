from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import unittest

spec = spec_from_file_location("release_assets", Path(__file__).resolve().parents[2] / "scripts/check-release-assets.py")
assets = module_from_spec(spec)
spec.loader.exec_module(assets)


class ReleaseAssetsTest(unittest.TestCase):
    def test_exact_names_missing_extra_and_duplicates(self):
        names = sorted(assets.expected_assets("1.2.3"))
        self.assertEqual(len(names), 15)
        self.assertEqual(assets.validate("1.2.3", names), ([], [], False))
        self.assertTrue(assets.validate("1.2.3", names[1:])[0])
        self.assertTrue(assets.validate("1.2.3", names + ["unexpected"])[1])
        self.assertTrue(assets.validate("1.2.3", names + [names[0]])[2])

    def test_checksum_manifest_covers_every_archive_once(self):
        archives = sorted(assets.expected_assets("1.2.3") - {"checksums.txt", "install.sh", "install.ps1"})
        lines = ["a" * 64 + "  " + name for name in archives]
        self.assertTrue(assets.validate_checksums("1.2.3", "\n".join(lines)))
        self.assertFalse(assets.validate_checksums("1.2.3", "\n".join(lines[:-1])))
        self.assertFalse(assets.validate_checksums("1.2.3", "\n".join(lines + [lines[0]])))
        self.assertFalse(assets.validate_checksums("1.2.3", "invalid"))
