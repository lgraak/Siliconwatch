"""Inspect a possibly interrupted release against an independently reviewed local candidate."""

import argparse
import json
import os
from pathlib import Path

from candidate import require, verify
from promote import GitHub, asset_differences, release_metadata

REPOSITORY = "lgraak/Siliconwatch"


def inspect(api, version, commit, expected, metadata, release_id=None):
    ref = api.json("git/ref/tags/" + version, missing_ok=True)
    tag_sha = ref["object"]["sha"] if ref else None
    issues = []
    if ref and (ref["ref"] != "refs/tags/" + version or tag_sha != commit):
        issues.append("Tag differs from the approved lightweight tag and commit")
    releases = [r for r in api.pages("releases") if r["tag_name"] == version]
    if release_id is not None:
        require(release_id > 0, "Expected a positive release ID")
        # The ID identifies the intended object; listing still detects competing drafts.
        selected = api.json(f"releases/{release_id}")
        if len(releases) != 1 or releases[0]["id"] != release_id or selected["id"] != release_id:
            return {"state": "ambiguous", "release_id": release_id, "tag_sha": tag_sha,
                    "issues": ["Release ID and visible same-version inventory do not uniquely agree"]}
        releases = [selected]
    if len(releases) > 1:
        issues.append("Multiple visible releases claim this version")
    if len(releases) != 1:
        state = "tag_only_or_release_hidden" if ref else "no_visible_tag_or_release"
        return {"state": "ambiguous" if issues else state, "tag_sha": tag_sha, "issues": issues}

    release = releases[0]
    if any(release.get(k) != v for k, v in metadata.items()):
        issues.append("Release metadata differs from the approved candidate")
    require(type(release.get("draft")) is bool, "Missing release visibility state")
    issues.extend(asset_differences(api.pages(f"releases/{release['id']}/assets"), expected))
    if release["draft"]:
        state = "draft_with_tag" if ref else "draft_without_tag"
    else:
        if release.get("immutable") is not True:
            issues.append("GitHub did not confirm this published release is immutable")
        if ref is None:
            issues.append("Published release has no visible tag")
        state = "public_mismatch" if issues else "public_verified"
    return {"state": state, "release_id": release["id"], "tag_sha": tag_sha, "issues": issues}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--digest", required=True, help="Manifest SHA-256 recorded at approval")
    parser.add_argument("--release-id", type=int, help="Draft ID printed by the publication run, if available")
    args = parser.parse_args()
    expected = verify(args.candidate, args.version, args.commit, args.digest)
    metadata = release_metadata(args.version, args.commit, args.candidate / "RELEASE-NOTES.md")
    api = GitHub(REPOSITORY, os.environ["GH_TOKEN"])
    result = inspect(api, args.version, args.commit, expected, metadata, args.release_id)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["state"] == "public_verified" else 1


if __name__ == "__main__":
    raise SystemExit(main())
