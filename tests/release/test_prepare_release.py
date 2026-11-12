import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipIf(os.name == "nt", "Release preparation runs on Linux CI")
class PrepareReleaseTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "scripts").mkdir()
        (self.root / "bin").mkdir()
        shutil.copy(ROOT / "scripts" / "prepare-release.sh", self.root / "scripts")
        shutil.copy(ROOT / "scripts" / "release-version.py", self.root / "scripts")
        (self.root / "scripts" / "check-release-consistency.sh").write_text("exit 0\n")
        (self.root / "Cargo.toml").write_text('[workspace.package]\nversion = "1.2.3"\n')
        gh = self.root / "bin" / "gh"
        gh.write_text('''#!/bin/sh
if [ "$1" = release ]; then
  if [ "$TEST_RELEASE_STATUS" = 0 ]; then printf '%s\\n' "${TEST_RELEASE_DRAFT:-false}"; fi
  exit "$TEST_RELEASE_STATUS"
fi
if [ "$1" = api ] && [ "$2" = -i ]; then printf 'HTTP/2 %s result\\n' "$TEST_HTTP_STATUS"; exit 1; fi
printf '%s\\n' "$*" >> "$TEST_CALLS"
''')
        gh.chmod(0o755)
        self.git("init", "-q")
        self.git("config", "user.name", "Release Test")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("add", ".")
        self.git("commit", "-qm", "fixture")
        self.source = self.git("rev-parse", "HEAD")

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, text=True).strip()

    def prepare(self, release_status, http_status, draft="false"):
        output = self.root / "output"
        env = dict(os.environ, PATH=f"{self.root / 'bin'}:{os.environ['PATH']}", GH_REPO="fixture/project",
                   GITHUB_OUTPUT=str(output), TEST_CALLS=str(self.root / "calls"),
                   TEST_RELEASE_STATUS=str(release_status), TEST_HTTP_STATUS=str(http_status), TEST_RELEASE_DRAFT=draft)
        result = subprocess.run(["bash", "scripts/prepare-release.sh"], cwd=self.root, env=env, capture_output=True, text=True)
        values = dict(line.split("=", 1) for line in output.read_text().splitlines()) if output.exists() else {}
        return result, values

    def test_existing_release_skips(self):
        result, values = self.prepare(0, 200)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(values["should_release"], "false")
        self.assertFalse((self.root / "calls").exists())

    def test_missing_release_creates_tag(self):
        result, values = self.prepare(1, 404)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(values["source_sha"], self.source)
        self.assertEqual(values["needs_github_release"], "true")
        self.assertIn(f"sha={self.source}", (self.root / "calls").read_text())

    def test_failed_lookup_is_not_treated_as_missing(self):
        result, values = self.prepare(1, 500)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(values)
        self.assertFalse((self.root / "calls").exists())

    def test_existing_tag_uses_tagged_source(self):
        self.git("tag", "v1.2.3")
        (self.root / "later").write_text("later")
        self.git("add", "later")
        self.git("commit", "-qm", "later")
        result, values = self.prepare(1, 404)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(values["source_sha"], self.source)
        self.assertFalse((self.root / "calls").exists())

    def test_release_checkout_refuses_unrelated_or_invalid_source(self):
        ancestor = self.source
        (self.root / "later").write_text("later")
        self.git("add", "later")
        self.git("commit", "-qm", "later")
        workflow = self.git("rev-parse", "HEAD")
        self.git("checkout", "--orphan", "unrelated")
        self.git("commit", "--allow-empty", "-qm", "unrelated")
        unrelated = self.git("rev-parse", "HEAD")
        script = ROOT / "scripts/checkout-release-source.sh"
        for source, expected in ((unrelated, 1), ("main; exit 0", 1), (ancestor, 0)):
            self.git("checkout", "--detach", workflow)
            env = dict(os.environ, RELEASE_SOURCE_SHA=source, RELEASE_WORKFLOW_SHA=workflow)
            result = subprocess.run(["bash", str(script)], cwd=self.root, env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, expected, result.stderr)
            self.assertEqual(self.git("rev-parse", "HEAD"), ancestor if expected == 0 else workflow)

    def test_existing_draft_rebuilds_its_tagged_source(self):
        self.git("tag", "v1.2.3")
        (self.root / "later").write_text("later")
        self.git("add", "later")
        self.git("commit", "-qm", "later")
        result, values = self.prepare(0, 200, draft="true")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(values["source_sha"], self.source)
        self.assertEqual(values["needs_github_release"], "true")
        self.assertFalse((self.root / "calls").exists())

    def test_draft_without_tag_refuses_to_create_one(self):
        result, values = self.prepare(0, 200, draft="true")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(values)
        self.assertFalse((self.root / "calls").exists())
