#!/usr/bin/env python3
"""Verify one sealed Actions candidate. Publication requires a separate approved dispatch."""

import argparse
import json
import os
import re
import shutil
import tempfile
import zipfile
from pathlib import Path
from urllib import error, request

from candidate import bundle_names, check_identity, require, sha256, verify
from envelope import MAX_BYTES, unpack as unpack_envelope

CANDIDATE_WORKFLOW = ".github/workflows/release-candidate.yml"
REPOSITORY = "lgraak/Siliconwatch"
MAINTAINER = "lgraak"
MAINTAINER_ID = 9121312


class DownloadRedirect(request.HTTPRedirectHandler):
    """GitHub redirects artifact downloads to short-lived HTTPS storage URLs."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        require(req.get_method() == "GET" and newurl.startswith("https://"),
                "Unexpected redirect from GitHub")
        redirected = super().redirect_request(req, fp, code, msg, headers, newurl)
        redirected.remove_header("Authorization")
        return redirected


class GitHub:
    def __init__(self, repo, token):
        self.repo = repo
        self.headers = {"Authorization": "Bearer " + token, "Accept": "application/vnd.github+json",
                        "X-GitHub-Api-Version": "2022-11-28", "User-Agent": "SiliconWatch-release"}
        self.opener = request.build_opener(DownloadRedirect())

    def open(self, path, method="GET", *, data=None, headers=None, upload=False, missing_ok=False):
        host = "uploads.github.com" if upload else "api.github.com"
        url = f"https://{host}/repos/{self.repo}/{path}"
        req = request.Request(url, data=data, method=method, headers=self.headers | (headers or {}))
        try:
            return self.opener.open(req, timeout=180)
        except error.HTTPError as exc:
            if missing_ok and exc.code == 404:
                return None
            raise ValueError(f"GitHub {method} {path} returned HTTP {exc.code}") from None

    def json(self, path, method="GET", *, body=None, missing_ok=False):
        response = self.open(path, method, data=None if body is None else json.dumps(body).encode(),
                             headers={"Content-Type": "application/json"}, missing_ok=missing_ok)
        if response is None:
            return None
        with response:
            return json.load(response)

    def pages(self, path, key=None):
        page = 1
        while True:
            separator = "&" if "?" in path else "?"
            result = self.json(f"{path}{separator}per_page=100&page={page}")
            rows = result[key] if key else result
            yield from rows
            if len(rows) < 100:
                return
            page += 1

    def download(self, artifact, destination):
        require(0 < artifact["size_in_bytes"] <= MAX_BYTES, "Invalid encrypted artifact size")
        with self.open(f"actions/artifacts/{artifact['id']}/zip") as response, destination.open("xb") as output:
            remaining = artifact["size_in_bytes"]
            while remaining:
                chunk = response.read(min(remaining, 2**20))
                require(chunk, "Truncated Actions artifact")
                output.write(chunk)
                remaining -= len(chunk)
            require(not response.read(1), "Actions artifact exceeds recorded size")
        require(artifact["digest"] == "sha256:" + sha256(destination), "Actions archive digest mismatch")

    def upload(self, release_id, path):
        with path.open("rb") as stream:
            headers = {"Content-Type": "application/octet-stream", "Content-Length": str(path.stat().st_size)}
            with self.open(f"releases/{release_id}/assets?name={path.name}", "POST", data=stream,
                           headers=headers, upload=True) as response:
                return json.load(response)


def trusted_artifact(api, repo, run_id, version, commit):
    run = api.json(f"actions/runs/{run_id}")
    expected = {"id": run_id, "event": "workflow_dispatch", "path": CANDIDATE_WORKFLOW,
                "head_branch": "main", "head_sha": commit, "run_attempt": 1, "status": "completed", "conclusion": "success"}
    require(all(run.get(key) == value for key, value in expected.items()), "Candidate run provenance mismatch")
    actor = run["actor"]
    require(actor["login"] == MAINTAINER and actor["id"] == MAINTAINER_ID,
            "Candidate requires a manual dispatch by lgraak")
    for key in ("repository", "head_repository"):
        require(run[key]["full_name"] == repo, "Candidate run repository mismatch")
    artifacts = [a for a in api.pages(f"actions/runs/{run_id}/artifacts", "artifacts")
                 if a["name"] == "candidate-" + version]
    require(len(artifacts) == 1 and artifacts[0]["expired"] is False, "Expected one live sealed candidate artifact")
    return artifacts[0]


def unpack(blob, directory, version, key):
    require(re.fullmatch(r"[a-fA-F0-9]{64}", key), "CANDIDATE_KEY must be a 64-character hex secret")
    with zipfile.ZipFile(blob) as archive:
        entries = archive.infolist()
        require(len(entries) == 1 and entries[0].filename == "candidate.tar.gpg"
                and 0 < entries[0].file_size <= MAX_BYTES, "Unexpected Actions artifact payload")
        encrypted = directory.parent / "candidate.tar.gpg"
        with archive.open(entries[0]) as stream, encrypted.open("xb") as output:
            shutil.copyfileobj(stream, output)
    unpack_envelope(encrypted, directory, key, expected=bundle_names(version))


def require_approval(api, repo, env):
    ref = "refs/heads/main"
    expected = {"GITHUB_REPOSITORY": REPOSITORY, "GITHUB_REPOSITORY_ID": "1459688906",
                "GITHUB_REPOSITORY_OWNER_ID": str(MAINTAINER_ID), "GITHUB_REF_PROTECTED": "true",
                "GITHUB_REF": ref, "GITHUB_EVENT_NAME": "workflow_dispatch",
                "GITHUB_WORKFLOW_REF": f"{repo}/.github/workflows/release-publish.yml@{ref}",
                "GITHUB_RUN_ATTEMPT": "1"}
    require(repo == REPOSITORY and all(env.get(key) == value for key, value in expected.items()),
            "Publish requires a fresh manual release-publish workflow from main")
    publisher_run = env.get("GITHUB_RUN_ID", "")
    require(re.fullmatch(r"[1-9][0-9]*", publisher_run), "Invalid publisher run ID")
    require(env.get("GITHUB_ACTOR") == MAINTAINER and env.get("GITHUB_ACTOR_ID") == str(MAINTAINER_ID),
            "Publication requires a manual dispatch by lgraak")
    reviews = api.json(f"actions/runs/{publisher_run}/approvals")
    decisions = [row for row in reviews if any(e["name"] == "release" for e in row["environments"])]
    require(len(decisions) == 1 and decisions[0]["state"] == "approved" and
            decisions[0]["user"]["login"] == MAINTAINER and decisions[0]["user"].get("id") == MAINTAINER_ID,
            "Expected one explicit release environment approval by lgraak for this run")


def check_release_name(api, version):
    require(api.json("git/ref/tags/" + version, missing_ok=True) is None, "Release tag already exists")
    require(not any(release["tag_name"] == version for release in api.pages("releases")),
            "A release or draft already owns this tag")


def asset_differences(assets, expected):
    seen = set()
    differences = []
    for asset in assets:
        name = asset["name"]
        if name in seen or name not in expected:
            differences.append(f"Unexpected or duplicate asset: {name}")
        else:
            item = expected[name]
            if (asset.get("state") != "uploaded" or asset.get("size") != item["bytes"]
                    or asset.get("digest") != "sha256:" + item["sha256"]):
                differences.append(f"Remote release asset digest or size mismatch: {name}")
        seen.add(name)
    differences.extend(f"Missing asset: {name}" for name in sorted(expected.keys() - seen))
    return differences


def verify_assets(api, release_id, expected):
    differences = asset_differences(api.pages(f"releases/{release_id}/assets"), expected)
    require(not differences, "Remote release asset verification failed: " + ", ".join(differences))


def release_metadata(version, commit, notes):
    return {"tag_name": version, "target_commitish": commit, "name": "SiliconWatch " + version,
            "body": notes.read_text(), "prerelease": "-" in version}


def promote(api, repo, version, commit, run_id, digest, publish, env, *, rehearse_approval=False):
    require(not (publish and rehearse_approval), "Choose either rehearsal or publication")
    check_identity(version, commit)
    require(re.fullmatch(r"[A-Za-z0-9-]+/[A-Za-z0-9_.-]+", repo), "Invalid repository")
    require(isinstance(run_id, int) and run_id > 0, "Invalid candidate run ID")
    require(re.fullmatch(r"[a-f0-9]{64}", digest), "Expected full approved manifest digest")
    key = env.get("CANDIDATE_KEY", "")
    require(re.fullmatch(r"[a-fA-F0-9]{64}", key), "CANDIDATE_KEY must be a 64-character hex secret")
    if publish or rehearse_approval:
        require_approval(api, repo, env)
    artifact = trusted_artifact(api, repo, run_id, version, commit)
    with tempfile.TemporaryDirectory() as temp:
        directory = Path(temp) / "bundle"
        zip_path = Path(temp) / "download.zip"
        api.download(artifact, zip_path)
        unpack(zip_path, directory, version, key)
        expected = verify(directory, version, commit, digest)
        check_release_name(api, version)
        print(f"Verified {version}, source {commit}, run {run_id}, artifact {artifact['id']}, manifest {digest}.")
        if not publish:
            return
        data = release_metadata(version, commit, directory / "RELEASE-NOTES.md") | {"draft": True}
        release = api.json("releases", "POST", body=data)
        require(all(release.get(k) == v for k, v in data.items()), "GitHub did not create the intended draft")
        release_id = release["id"]
        print(f"Draft {release_id} created. If interrupted, reconcile the release ID, tag and all assets before any retry.")
        for name in sorted(expected):
            api.upload(release_id, directory / name)
        verify_assets(api, release_id, expected)
        draft = api.json(f"releases/{release_id}")
        require(all(draft.get(k) == v for k, v in data.items()), "Draft release metadata changed during upload")
        # Creating a new ref refuses existing tags atomically. Updating refs could overwrite prior releases.
        ref = api.json("git/refs", "POST", body={"ref": "refs/tags/" + version, "sha": commit})
        require(ref["object"]["sha"] == commit and ref["ref"] == "refs/tags/" + version, "Unexpected release tag")
        published = api.json(f"releases/{release_id}", "PATCH", body={"draft": False})
        approved = data | {"draft": False}
        require(all(published.get(k) == v for k, v in approved.items()), "Published release metadata mismatch")
        final = api.json(f"releases/{release_id}")
        require(final.get("immutable") is True and all(final.get(k) == v for k, v in approved.items()),
                "Release may already be public; reconcile its metadata and immutability")
        require(api.json("git/ref/tags/" + version)["object"]["sha"] == commit, "Published tag mismatch")
        verify_assets(api, release_id, expected)
        print(f"Published and verified {published['html_url']}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ("repository", "version", "commit", "digest"):
        parser.add_argument("--" + arg, required=True)
    parser.add_argument("--run-id", required=True, type=int)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--publish", action="store_true")
    mode.add_argument("--rehearse-approval", action="store_true", help="verify approval and candidate with read-only access")
    args = parser.parse_args()
    api = GitHub(args.repository, os.environ["GH_TOKEN"])
    promote(api, args.repository, args.version, args.commit, args.run_id, args.digest, args.publish, os.environ,
            rehearse_approval=args.rehearse_approval)


if __name__ == "__main__":
    main()
