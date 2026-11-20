"""Regression checks for the release workflow's triggers, privileges and version selection."""
from pathlib import Path
from shutil import copyfile
import subprocess
import tempfile
import tomllib
import unittest

import yaml

from check_release_version import check

ROOT = Path(__file__).resolve().parents[2]


def workflow(name):
    return yaml.load((ROOT / ".github/workflows" / name).read_text(), Loader=yaml.BaseLoader)


class ReleaseWorkflowTests(unittest.TestCase):
    def test_build_and_candidate_jobs_are_read_only(self):
        quality = workflow("alpha1.yml")
        self.assertIn("workflow_call", quality["on"])
        self.assertEqual(quality["permissions"], {"contents": "read"})
        candidate = workflow("release-candidate.yml")
        self.assertEqual(set(candidate["on"]), {"workflow_dispatch"})
        self.assertEqual(candidate["permissions"], {"contents": "read"})
        for job in candidate["jobs"].values():
            self.assertNotIn("write", job.get("permissions", {}).values())
        self.assertEqual(set(candidate["jobs"]["seal"]["needs"]), {"quality", "source", "package"})

    def test_only_the_explicitly_approved_job_can_publish(self):
        approval = workflow("release-publish.yml")
        self.assertEqual(set(approval["on"]), {"workflow_dispatch"})
        self.assertEqual(approval["on"]["workflow_dispatch"]["inputs"]["publish"]["default"], "false")
        self.assertTrue(all(value == "read" for value in approval["permissions"].values()))
        jobs = approval["jobs"]
        writers = [name for name, job in jobs.items() if "write" in job.get("permissions", {}).values()]
        self.assertEqual(writers, ["publish"])
        publish = jobs["publish"]
        self.assertEqual(set(publish["needs"]), {"verify", "publication-policy"})
        self.assertIn("inputs.publish", publish["if"])
        self.assertIn("refs/heads/main", publish["if"])
        self.assertEqual(publish["environment"]["name"], "release")
        self.assertNotIn("--publish", str(jobs["verify"]["steps"]))
        self.assertIn("--publish", str(publish["steps"]))

    def test_approval_rehearsal_requires_publication_policy_and_read_only_release_job(self):
        approval = workflow("release-publish.yml")
        flags = approval["on"]["workflow_dispatch"]["inputs"]
        for flag in ("rehearse_approval", "publish"):
            self.assertEqual((flags[flag]["type"], flags[flag]["default"]), ("boolean", "false"))
        jobs = approval["jobs"]
        self.assertEqual(jobs["publication-policy"]["if"], "inputs.publish != inputs.rehearse_approval")
        rehearsal, publish = jobs["rehearse"], jobs["publish"]
        self.assertEqual(set(rehearsal["needs"]), {"verify", "publication-policy"})
        self.assertEqual(rehearsal["environment"], publish["environment"])
        self.assertEqual(rehearsal["permissions"], {"contents": "read", "actions": "read"})
        self.assertTrue(rehearsal["if"].startswith("inputs.rehearse_approval && !inputs.publish && "))
        self.assertTrue(publish["if"].startswith("inputs.publish && !inputs.rehearse_approval && "))
        self.assertIn("--rehearse-approval", str(rehearsal["steps"]))
        self.assertNotIn("--publish", str(rehearsal["steps"]))

        gate = jobs["verify"]["steps"][0]
        self.assertEqual(gate["env"], {"PUBLISH": "${{ inputs.publish }}",
                                       "REHEARSE_APPROVAL": "${{ inputs.rehearse_approval }}"})
        for requests, allowed in [(("false", "false"), True), (("false", "true"), True),
                                  (("true", "false"), True), (("true", "true"), False), (("", ""), False)]:
            with self.subTest(requests=requests):
                env = {"GITHUB_REPOSITORY": "lgraak/Siliconwatch", "GITHUB_REF": "refs/heads/main",
                       "PUBLISH": requests[0], "REHEARSE_APPROVAL": requests[1]}
                result = subprocess.run(["bash", "-e", "-c", gate["run"]], env=env, capture_output=True)
                self.assertEqual(result.returncode == 0, allowed)

    def test_live_policy_is_checked_before_the_release_environments(self):
        policy = workflow("release-policy.yml")
        self.assertEqual(set(policy["on"]), {"workflow_call"})
        self.assertEqual(policy["permissions"], {"contents": "read"})
        guard, settings = (policy["jobs"][name] for name in ("origin", "settings"))
        self.assertNotIn("environment", guard)
        self.assertNotIn("secrets.", str(guard))
        for field in ("GITHUB_REF_PROTECTED", "GITHUB_EVENT_NAME", "GITHUB_WORKFLOW_REF", "GITHUB_RUN_ATTEMPT"):
            self.assertIn(field, str(guard["steps"]))
        self.assertEqual(settings["needs"], "origin")
        self.assertEqual(settings["environment"], "release-policy")
        self.assertIn("secrets.RELEASE_POLICY_READER", str(settings))
        self.assertNotIn("secrets.CANDIDATE_KEY", str(policy))
        self.assertEqual(policy["on"]["workflow_call"]["secrets"], {"RELEASE_POLICY_READER": {"required": "true"}})
        for filename in ("release-candidate.yml", "release-publish.yml"):
            jobs = workflow(filename)["jobs"]
            self.assertEqual(jobs["policy"]["uses"], "./.github/workflows/release-policy.yml")
            for entry in jobs.values():
                if entry.get("uses") == "./.github/workflows/release-policy.yml":
                    self.assertEqual(entry["secrets"], {"RELEASE_POLICY_READER": "${{ secrets.RELEASE_POLICY_READER }}"})
                else:
                    self.assertNotIn("RELEASE_POLICY_READER", str(entry))
                if "environment" in entry:
                    self.assertIn("github.run_attempt == '1'", entry["if"])
        jobs = workflow("release-candidate.yml")["jobs"]
        self.assertEqual(jobs["source"]["needs"], "policy")
        self.assertEqual(jobs["package"]["needs"], "source")
        jobs = workflow("release-publish.yml")["jobs"]
        self.assertEqual(jobs["verify"]["needs"], "policy")
        self.assertEqual(jobs["publication-policy"]["with"]["publication"], "true")
        self.assertEqual(jobs["publication-policy"]["needs"], "verify")
        self.assertEqual(jobs["publication-policy"]["uses"], "./.github/workflows/release-policy.yml")

    def test_dispatch_guard_rejects_unprotected_other_and_retried_workflows(self):
        script = workflow("release-policy.yml")["jobs"]["origin"]["steps"][0]["run"]
        env = {"GITHUB_REPOSITORY_ID": "1459688906", "GITHUB_REPOSITORY": "lgraak/Siliconwatch",
               "GITHUB_ACTOR": "lgraak", "GITHUB_ACTOR_ID": "9121312",
               "GITHUB_EVENT_NAME": "workflow_dispatch", "GITHUB_REF": "refs/heads/main",
               "GITHUB_REF_PROTECTED": "true", "GITHUB_RUN_ATTEMPT": "1"}
        for filename in ("release-candidate.yml", "release-publish.yml"):
            env["GITHUB_WORKFLOW_REF"] = f"lgraak/Siliconwatch/.github/workflows/{filename}@refs/heads/main"
            subprocess.run(["bash", "-e", "-c", script], env=env, check=True)
        for name, value in (("GITHUB_REPOSITORY_ID", "0"), ("GITHUB_REPOSITORY", "other/Siliconwatch"),
                            ("GITHUB_ACTOR", "lgraakcg"), ("GITHUB_ACTOR_ID", "503"),
                            ("GITHUB_REF_PROTECTED", "false"), ("GITHUB_REF", "refs/heads/topic"),
                            ("GITHUB_EVENT_NAME", "push"), ("GITHUB_RUN_ATTEMPT", "2"),
                            ("GITHUB_WORKFLOW_REF", "lgraak/Siliconwatch/.github/workflows/other.yml@refs/heads/main")):
            with self.subTest(name=name):
                result = subprocess.run(["bash", "-e", "-c", script], env=env | {name: value})
                self.assertNotEqual(result.returncode, 0)

    def test_version_must_match_both_manifests_lock_entries_and_notes(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for rel in ("Cargo.toml", "gui/Cargo.toml", "Cargo.lock"):
                (root / rel).parent.mkdir(exist_ok=True)
                copyfile(ROOT / rel, root / rel)
            version = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
            tag = "v" + version
            with self.assertRaises(ValueError):
                check(tag, root)
            notes = root / "docs/releases" / (tag + ".md")
            notes.parent.mkdir(parents=True)
            notes.touch()
            check(tag, root)
            with self.assertRaises(ValueError):
                check(tag + ".unexpected", root)
            manifest = root / "gui/Cargo.toml"
            manifest.write_text(manifest.read_text().replace(version, "1.2.3", 1))
            with self.assertRaises(ValueError):
                check(tag, root)


if __name__ == "__main__":
    unittest.main()
