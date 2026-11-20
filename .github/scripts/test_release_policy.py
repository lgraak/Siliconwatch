"""Fail-closed contract checks for the release policy reader, without credentials or network."""
import copy
import unittest
from unittest.mock import patch

from check_release_policy import ACTIONS_APP_ID, OWNER_ID, REPO, REPO_ID, check_policy, inventory, read_api


def page(key, rows):
    return [{"total_count": len(rows), key: rows}]


def hosted_settings():
    owner = {"type": "User", "id": OWNER_ID, "login": "lgraak"}
    envs = ["release", "release-candidate", "release-policy", "other env"]
    responses = {
        "": {"id": REPO_ID, "full_name": REPO, "default_branch": "main", "owner": owner},
        "/actions/permissions/workflow": {"default_workflow_permissions": "read", "can_approve_pull_request_reviews": False},
        "/immutable-releases": {"enabled": True},
        "/branches/main": {"name": "main", "protected": True},
        "/branches/main/protection": {
            "enforce_admins": {"enabled": True}, "allow_force_pushes": {"enabled": False},
            "allow_deletions": {"enabled": False},
            "required_pull_request_reviews": {"dismiss_stale_reviews": False, "require_last_push_approval": False,
                                             "require_code_owner_reviews": False, "required_approving_review_count": 0},
            "required_status_checks": {"strict": True,
                                       "checks": [{"context": name, "app_id": ACTIONS_APP_ID}
                                                  for name in ("Rust 1.85.0", "DCO signoffs")]},
        },
        "/actions/secrets?per_page=100": page("secrets", []),
        "/environments?per_page=100": page("environments", [{"name": name} for name in envs]),
    }
    for env in envs:
        path = "/environments/" + env.replace(" ", "%20")
        responses[path] = {"name": env, "can_admins_bypass": False,
                           "protection_rules": [{"type": "branch_policy"}],
                           "deployment_branch_policy": {"protected_branches": False, "custom_branch_policies": True}}
        responses[path + "/deployment-branch-policies?per_page=100"] = page("branch_policies",
                                                                            [{"name": "main", "type": "branch"}])
        keys = {"release": ["CANDIDATE_KEY"], "release-candidate": ["CANDIDATE_KEY"],
                "release-policy": ["RELEASE_POLICY_READER"], "other env": ["OTHER"]}
        responses[path + "/secrets?per_page=100"] = page("secrets", [{"name": name} for name in keys[env]])
    responses["/environments/release"]["protection_rules"].append({
        "type": "required_reviewers", "prevent_self_review": False, "reviewers": [{"type": "User", "reviewer": owner}]
    })
    return responses


class HostedPolicyTests(unittest.TestCase):
    def setUp(self):
        self.settings = hosted_settings()

    def read(self, path, *, paginate=False):
        self.assertEqual(paginate, path.endswith("?per_page=100"))
        return self.settings[path]

    def rejects(self, path, nested, value):
        prior = copy.deepcopy(self.settings[path])
        try:
            row = self.settings[path]
            for key in nested[:-1]:
                row = row[key]
            row[nested[-1]] = value
            with self.assertRaises((ValueError, KeyError, TypeError)):
                check_policy(self.read)
        finally:
            self.settings[path] = prior

    def test_full_host_configuration_passes(self):
        check_policy(self.read)

    def test_actions_defaults_are_read_only_and_publication_requires_immutability(self):
        self.rejects("/actions/permissions/workflow", ["default_workflow_permissions"], "write")
        self.rejects("/actions/permissions/workflow", ["can_approve_pull_request_reviews"], True)
        check_policy(self.read, publication=True)
        for setting in ({"enabled": False}, {}):
            self.settings["/immutable-releases"] = setting
            check_policy(self.read)
            with self.assertRaises((ValueError, KeyError)):
                check_policy(self.read, publication=True)

    def test_personal_owner_repository_and_default_branch_are_bound(self):
        for keys, value in ((["id"], 7), (["owner", "type"], "Organization"), (["owner", "id"], 3),
                            (["default_branch"], "other"), (["full_name"], "other/Siliconwatch")):
            with self.subTest(keys=keys):
                self.rejects("", keys, value)

    def test_insufficient_main_policy_refuses_candidate(self):
        self.rejects("/branches/main", ["protected"], False)
        for keys, value in (
            (["enforce_admins", "enabled"], False), (["allow_force_pushes", "enabled"], True),
            (["allow_deletions", "enabled"], True),
            (["required_pull_request_reviews", "bypass_pull_request_allowances"], {"teams": [1], "apps": [], "users": []}),
            (["required_status_checks", "strict"], False), (["required_status_checks", "checks"], []),
            (["required_status_checks", "checks", 0, "app_id"], -1),
            (["required_status_checks", "checks", 1, "context"], "other check"),
        ):
            with self.subTest(keys=keys):
                self.rejects("/branches/main/protection", keys, value)

    def test_solo_main_requires_a_pr_without_a_second_approver(self):
        protection = "/branches/main/protection"
        review = "required_pull_request_reviews"
        for value in (None, {}):
            self.rejects(protection, [review], value)
        for field, value in (("require_code_owner_reviews", True), ("require_last_push_approval", True),
                             ("required_approving_review_count", 1), ("required_approving_review_count", False)):
            with self.subTest(field=field, value=value):
                self.rejects(protection, [review, field], value)
        del self.settings[protection][review]
        with self.assertRaisesRegex(ValueError, "require a pull request"):
            check_policy(self.read)

    def test_branch_or_tag_policy_must_be_unambiguous_on_every_environment(self):
        for name in ("release", "release-candidate", "release-policy"):
            path = "/environments/" + name
            for branch in ({"type": "tag", "name": "main"}, {"type": "branch", "name": "*"}, {"name": "main"}):
                with self.subTest(name=name, branch=branch):
                    self.rejects(path + "/deployment-branch-policies?per_page=100", [0, "branch_policies", 0], branch)
            self.rejects(path, ["deployment_branch_policy"], None)
            self.rejects(path, ["deployment_branch_policy", "protected_branches"], True)
            self.rejects(path + "/deployment-branch-policies?per_page=100", [0, "total_count"], 2)

    def test_review_identity_and_admin_bypass_must_be_proven(self):
        path = "/environments/release"
        for keys, value in ((["protection_rules", 1, "prevent_self_review"], True),
                            (["protection_rules", 1, "prevent_self_review"], None),
                            (["protection_rules", 1, "reviewers", 0, "type"], "Team"),
                            (["protection_rules", 1, "reviewers", 0, "reviewer", "login"], "lgraakcg"),
                            (["protection_rules", 1, "reviewers", 0, "reviewer", "id"], 7),
                            (["protection_rules", 1, "reviewers"], []), (["protection_rules"], [])):
            with self.subTest(keys=keys):
                self.rejects(path, keys, value)

    def test_release_requires_positive_evidence_of_disabled_administrator_bypass(self):
        release = self.settings["/environments/release"]
        release["can_admins_bypass"] = False
        check_policy(self.read, publication=True)
        for response in ({}, {"can_admins_bypass": True}, {"can_admins_bypass": None},
                         {"can_admins_bypass": 0}):
            release.pop("can_admins_bypass", None)
            release.update(response)
            with self.subTest(response=response), self.assertRaisesRegex(ValueError, "release.*can_admins_bypass=false"):
                check_policy(self.read, publication=True)

    def test_all_sensitive_names_must_have_only_the_permitted_scopes(self):
        paths = ("/actions", "/environments/release", "/environments/release-policy",
                 "/environments/release-candidate", "/environments/other%20env")
        for path in paths:
            key = path + "/secrets?per_page=100"
            correct = self.settings[key]
            for name in ("CANDIDATE_KEY", "RELEASE_POLICY_READER"):
                rows = [row for row in correct[0]["secrets"] if row["name"] != name]
                if rows == correct[0]["secrets"]:
                    rows = [*rows, {"name": name.lower()}]
                self.settings[key] = page("secrets", rows)
                with self.subTest(path=path, name=name), self.assertRaises(ValueError):
                    check_policy(self.read)
                self.settings[key] = correct

    def test_missing_or_unreadable_environment_cannot_disappear_from_the_gate(self):
        path = "/environments?per_page=100"
        self.settings[path] = page("environments", [{"name": "release-policy"}, {"name": "release-candidate"}])
        with self.assertRaisesRegex(ValueError, "Missing release environment"):
            check_policy(self.read)
        def denied(path):
            raise PermissionError(path)
        with self.assertRaises(PermissionError):
            check_policy(denied)

    def test_truncated_pagination_and_ambiguous_counts_are_refused(self):
        def collect(pages):
            return inventory(lambda *args, **kwargs: pages, "/secrets", "secrets")
        rows = [{"name": "A"}, {"name": "B"}]
        self.assertEqual(collect([{"total_count": 2, "secrets": [row]} for row in rows]), rows)
        for pages in ([], [{"total_count": 3, "secrets": rows}], [{"total_count": True, "secrets": [rows[0]]}],
                      [{"total_count": 2, "secrets": [rows[0]]}, {"total_count": 1, "secrets": [rows[1]]}]):
            with self.subTest(pages=pages), self.assertRaises(ValueError):
                collect(pages)

    def test_transport_is_read_only_and_never_prints_api_failure_content(self):
        with patch("check_release_policy.subprocess.run") as run:
            run.return_value.returncode = 1
            run.return_value.stderr = b"private service error"
            with self.assertRaisesRegex(ValueError, "metadata is unavailable") as caught:
                read_api("/actions/secrets", paginate=True)
            self.assertNotIn("private", str(caught.exception))
            args = run.call_args.args[0]
            self.assertEqual(args[args.index("--method") + 1], "GET")
            self.assertEqual(args[args.index("--hostname") + 1], "github.com")
            self.assertIn("--slurp", args)


if __name__ == "__main__":
    unittest.main()
