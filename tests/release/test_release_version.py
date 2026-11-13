from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import unittest

spec = spec_from_file_location("release_version", Path(__file__).parents[2] / "scripts" / "release-version.py")
module = module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseVersionTest(unittest.TestCase):
    def test_workspace_section_and_line_endings(self):
        source = '[package]\nversion = "9.9.9"\n[workspace.package]\nversion = "1.2.3"\n[dependencies]\nversion = "8.8.8"\n'
        for text in (source, source.replace("\n", "\r\n")):
            self.assertEqual(module.release_version(text), "1.2.3")

    def test_rejects_missing_or_invalid_version(self):
        for version in ("", "1.2", "v1.2.3", "1.2.3;echo bad", "01.2.3"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                module.release_version(f'[workspace.package]\nversion = "{version}"\n')
        with self.assertRaises(ValueError):
            module.release_version('[package]\nversion = "1.2.3"\n')


if __name__ == "__main__":
    unittest.main()
