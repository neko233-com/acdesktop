from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import unittest


spec = spec_from_file_location("ffi_header", Path(__file__).resolve().parents[2] / "scripts/check-ffi-header.py")
header = module_from_spec(spec)
spec.loader.exec_module(header)


class FfiHeaderCheckTest(unittest.TestCase):
    def test_line_endings_are_ignored_but_header_changes_are_detected(self):
        self.assertFalse(header.compare_headers("one\r\ntwo\r\n", "one\ntwo\n"))
        self.assertTrue(header.compare_headers("one\ntwo\n", "one\nthree\n"))
