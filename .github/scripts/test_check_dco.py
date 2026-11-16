"""Isolated Git fixtures for the contribution range and signoff format."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from check_dco import check_range, has_signoff


SIGNOFF = "Signed-off-by: Fixture Contributor <contributor@example.invalid>"


class SignoffTests(unittest.TestCase):
    def test_accepts_final_trailers_and_certified_onward_contribution(self):
        for message in [
            "Subject\n\n" + SIGNOFF,
            "Subject\n\nBody\n---\nMore body\n\n" + SIGNOFF + "\n\n",
            "Subject\n\nCo-authored-by: Other <other@example.invalid>\n" + SIGNOFF,
            "Subject\n\nsigned-off-by: Zoë Contributor <zoe@example.invalid>",
        ]:
            with self.subTest(message=message):
                self.assertTrue(has_signoff(message))

    def test_rejects_missing_malformed_and_body_mentions(self):
        for trailer in [
            "",
            "Signed-off-by: <nobody@example.invalid>",
            "Signed-off-by: Someone <not-an-email>",
            "Signed-off-by: Someone <a b@example.invalid>",
            "Signed-off-by: Someone <test\x1b@example.invalid>",
            "> " + SIGNOFF,
            "Example: " + SIGNOFF,
            SIGNOFF + "\nUnrelated prose",
            SIGNOFF + "\n\nLater body paragraph",
        ]:
            with self.subTest(trailer=trailer):
                self.assertFalse(has_signoff("Subject\n\n" + trailer))
        self.assertFalse(has_signoff(SIGNOFF))


class HistoryTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="siliconwatch-dco-")
        self.addCleanup(self.directory.cleanup)
        self.repo = Path(self.directory.name)
        self.env = patch.dict(os.environ, {
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_AUTHOR_NAME": "Fixture Author",
            "GIT_AUTHOR_EMAIL": "author@example.invalid",
            "GIT_COMMITTER_NAME": "Fixture Committer",
            "GIT_COMMITTER_EMAIL": "committer@example.invalid",
        })
        self.env.start()
        self.addCleanup(self.env.stop)
        self.git("init", "--quiet", "--initial-branch=main", "--template=")
        self.git("config", "commit.gpgsign", "false")
        self.base = self.commit("Unsigned existing base")

    def git(self, *args):
        return subprocess.check_output(
            ["git", *args], cwd=self.repo, text=True, stderr=subprocess.PIPE
        ).strip()

    def commit(self, message, signed=False):
        if signed:
            message += "\n\n" + SIGNOFF
        self.git("commit", "--quiet", "--allow-empty", "-m", message)
        return self.git("rev-parse", "HEAD")

    def test_signed_range_excludes_existing_unsigned_base(self):
        first = self.commit("First", signed=True)
        head = self.commit("Second", signed=True)
        self.assertEqual(check_range(self.base, head, self.repo), ([first, head], []))

    def test_signed_tip_does_not_hide_unsigned_contribution(self):
        unsigned = self.commit("Missing signoff")
        head = self.commit("Signed tip", signed=True)
        self.assertEqual(check_range(self.base, head, self.repo)[1], [unsigned])

    def test_side_parent_and_real_merge_checked_but_synthetic_merge_excluded(self):
        self.git("checkout", "--quiet", "-b", "contribution")
        self.commit("Signed contribution", signed=True)
        self.git("checkout", "--quiet", "-b", "side", self.base)
        side = self.commit("Unsigned side contribution")
        self.git("checkout", "--quiet", "contribution")
        self.git("merge", "--quiet", "--no-ff", "side", "-m", "Unsigned real merge")
        head = self.git("rev-parse", "HEAD")
        self.git("checkout", "--quiet", "main")
        newer_base = self.commit("Unrelated unsigned base update")
        self.git("merge", "--quiet", "--no-ff", "contribution", "-m", "Synthetic PR merge")
        commits, failures = check_range(newer_base, head, self.repo)
        self.assertIn(side, failures)
        self.assertIn(head, failures)
        self.assertNotIn(newer_base, commits)
        self.assertNotIn(self.git("rev-parse", "HEAD"), commits)

    def test_invalid_missing_empty_and_shallow_history_fail_closed(self):
        head = self.commit("Contribution", signed=True)
        for base, tip in [(self.base, self.base), ("--all", head), ("f" * 40, head)]:
            with self.subTest(base=base, head=tip):
                with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                    check_range(base, tip, self.repo)
        (self.repo / ".git" / "shallow").write_text(head + "\n")
        with self.assertRaisesRegex(ValueError, "Full commit history"):
            check_range(self.base, head, self.repo)


if __name__ == "__main__":
    unittest.main()
