"""Read the effective GitHub release settings before scheduling protected jobs."""
import argparse
import json
import os
import subprocess
from urllib.parse import quote

REPO = "lgraak/Siliconwatch"
REPO_ID = 1459688906
OWNER_ID = 9121312
ACTIONS_APP_ID = 15368
SCOPED_SECRETS = {
    "release-policy": {"RELEASE_POLICY_READER"},
    "release-candidate": {"CANDIDATE_KEY"},
    "release": {"CANDIDATE_KEY"},
}
SENSITIVE_NAMES = set.union(*SCOPED_SECRETS.values())


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def read_api(path, *, paginate=False):
    # The endpoint and HTTP method are fixed by this file. The reader's permissions
    # and environment must be restricted in GitHub; it can only read metadata.
    command = ["gh", "api", "--hostname", "github.com", "--method", "GET", "--header",
               "X-GitHub-Api-Version: 2022-11-28"]
    if paginate:
        command += ["--paginate", "--slurp"]
    result = subprocess.run([*command, "repos/" + REPO + path], capture_output=True, timeout=60)
    require(result.returncode == 0, f"GitHub policy metadata is unavailable for {path or '/'}")
    return json.loads(result.stdout)


def inventory(read, path, field):
    pages = read(path + "?per_page=100", paginate=True)
    require(isinstance(pages, list) and pages, f"Incomplete {field} response")
    total = pages[0]["total_count"]
    require(type(total) is int and total >= 0, f"Invalid {field} count")
    values = []
    for page in pages:
        require(page["total_count"] == total and isinstance(page[field], list), f"Inconsistent {field} listing")
        values.extend(page[field])
    require(len(values) == total, f"Incomplete {field} listing")
    return values


def names(rows):
    result = [row["name"] for row in rows]
    require(all(isinstance(name, str) and name for name in result), "Invalid GitHub inventory name")
    require(len(result) == len(set(result)), "Duplicate GitHub inventory entry")
    return set(result)


def check_main(read):
    # This verifier supports classic main protection on this personal repository.
    # Additional rulesets can strengthen it. A ruleset-only installation needs review.
    branch = read("/branches/main")
    require(branch["name"] == "main" and branch["protected"] is True, "main is unprotected")
    rules = read("/branches/main/protection")
    require(rules["enforce_admins"]["enabled"] is True and
            rules["allow_force_pushes"]["enabled"] is False and rules["allow_deletions"]["enabled"] is False,
            "main must apply protection to administrators and disallow forced updates and deletion")
    review = rules["required_pull_request_reviews"]
    require(type(review["required_approving_review_count"]) is int and review["required_approving_review_count"] >= 1
            and review["dismiss_stale_reviews"] is True and review["require_code_owner_reviews"] is True,
            "main requires a current pull request approval and code-owner review")
    # GitHub only supports PR bypass lists for organization-owned repositories.
    if "bypass_pull_request_allowances" in review:
        bypass = review["bypass_pull_request_allowances"]
        require(all(bypass[key] == [] for key in ("users", "teams", "apps")), "Unexpected main review bypass")
    status = rules["required_status_checks"]
    require(status["strict"] is True, "main must require up-to-date checks")
    for name in ("Rust 1.85.0", "DCO signoffs"):
        apps = [check["app_id"] for check in status["checks"] if check["context"] == name]
        require(apps == [ACTIONS_APP_ID], f"Missing or ambiguous GitHub Actions check: {name}")


def check_environment(read, name, owner_id):
    env_path = "/environments/" + name
    env = read(env_path)
    require(env["name"] == name, "Unexpected environment identity")
    selection = env["deployment_branch_policy"]
    require(selection["protected_branches"] is False and selection["custom_branch_policies"] is True,
            f"{name} must use selected branches")
    branches = inventory(read, env_path + "/deployment-branch-policies", "branch_policies")
    require(len(branches) == 1 and branches[0]["name"] == "main" and branches[0]["type"] == "branch",
            f"{name} must allow exactly the main branch")
    if name == "release":
        require(env["can_admins_bypass"] is False, "Administrator bypass must be disabled on release")
        reviewers = [rule for rule in env["protection_rules"] if rule["type"] == "required_reviewers"]
        require(len(reviewers) == 1 and reviewers[0]["prevent_self_review"] is True,
                "release requires independent reviewer approval")
        allowed = reviewers[0]["reviewers"]
        require(len(allowed) == 1 and allowed[0]["type"] == "User" and
                allowed[0]["reviewer"]["id"] == owner_id and allowed[0]["reviewer"]["login"] == "lgraak",
                "Only the designated release reviewer may approve")


def check_policy(read, *, publication=False):
    repo = read("")
    require(repo["id"] == REPO_ID and repo["full_name"] == REPO and repo["default_branch"] == "main" and
            repo["owner"]["type"] == "User" and repo["owner"]["id"] == OWNER_ID, "Unexpected repository or owner")
    # The fixed personal owner has no organization secret fallback. Read the whole
    # repository list; a 404 might mean a permission failure, not an absent secret.
    check_main(read)
    defaults = read("/actions/permissions/workflow")
    require(defaults["default_workflow_permissions"] == "read" and defaults["can_approve_pull_request_reviews"] is False,
            "Repository Actions permissions must default to read and disallow PR approvals")
    if publication:
        require(read("/immutable-releases")["enabled"] is True, "Publication requires immutable releases")
    broad = names(inventory(read, "/actions/secrets", "secrets"))
    require(not {name.upper() for name in broad} & SENSITIVE_NAMES, "Release secret configured at repository scope")
    environments = names(inventory(read, "/environments", "environments"))
    require(SCOPED_SECRETS.keys() <= environments, "Missing release environment")
    for name in sorted(environments):
        require(name not in (".", ".."), "Unexpected environment name")
        path = "/environments/" + quote(name, safe="")
        scoped = names(inventory(read, path + "/secrets", "secrets"))
        actual = {secret.upper() for secret in scoped} & SENSITIVE_NAMES
        require(actual == SCOPED_SECRETS.get(name, set()), "Missing or misplaced release secret")
    for name in SCOPED_SECRETS:
        check_environment(read, name, OWNER_ID)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("publication", choices=("true", "false"), help="also require immutable releases")
    args = parser.parse_args()
    try:
        require(os.environ.get("GH_TOKEN", "").startswith("github_pat_"),
                "A repository-scoped fine-grained release policy reader is required")
        check_policy(read_api, publication=args.publication == "true")
    except (ValueError, KeyError, TypeError, OSError, IndexError, subprocess.SubprocessError) as exc:
        raise SystemExit(f"Release policy verification failed: {exc}") from None
    print("Verified main protection, deployment restrictions, reviewer policy and scoped secret names.")
