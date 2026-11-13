from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = spec_from_file_location("release_policy", ROOT / "scripts/check-release-policy.py")
policy = module_from_spec(spec)
spec.loader.exec_module(policy)


class ReleasePolicyTest(unittest.TestCase):
    def test_current_policy_and_mutations(self):
        source = (ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8")
        self.assertEqual(policy.violations(source), [])
        for old, new in (("- target: aarch64-apple-darwin", "- target: missing"),
                         ("ffi-release-panic-guard, linux-archive-contents]", "linux-archive-contents]"),
                         ("check-release-assets.py local", "missing-asset-check.py local"),
                         ("windows-msi-selftest.ps1", "missing-msi-selftest.ps1"),
                         ("actions/attest-build-provenance@", "invalid-attestation@"),
                         ("--draft=false", "--draft=true"),
                         ("scripts/checkout-release-source.sh", "scripts/missing.sh")):
            with self.subTest(rule=old):
                self.assertIn(old, source)
                self.assertTrue(policy.violations(source.replace(old, new, 1)))
