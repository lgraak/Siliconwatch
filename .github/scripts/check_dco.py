"""Check for well-formed DCO attestations in every commit contributed by a PR."""

import argparse
import re
import subprocess
import sys


def has_signoff(message):
    """Accept a conventional final trailer paragraph, never a body substring."""
    paragraphs = re.split(r"\n[ \t]*\n", message.replace("\r\n", "\n").rstrip())
    if len(paragraphs) < 2:
        return False
    trailers = paragraphs[-1].splitlines()
    if not all(re.fullmatch(r"[A-Za-z0-9-]+:[ \t]*.*", line) for line in trailers):
        return False
    for line in trailers:
        match = re.fullmatch(
            r"Signed-off-by:[ \t]+([^<>]+)[ \t]+<([^<>\s@]+@[^<>\s@]+)>",
            line,
            flags=re.IGNORECASE,
        )
        if match and match[1].strip() and all(c.isprintable() for c in match[1] + match[2]):
            return True
    return False


def git(*args, cwd=None):
    return subprocess.check_output(
        ["git", *args], cwd=cwd, text=True, encoding="utf-8", stderr=subprocess.PIPE
    )


def check_range(base, head, cwd=None):
    for sha in (base, head):
        if not re.fullmatch(r"[0-9a-fA-F]{40}", sha):
            raise ValueError("Expected full 40-character commit SHAs")
        git("rev-parse", "--verify", sha + "^{commit}", cwd=cwd)
    if git("rev-parse", "--is-shallow-repository", cwd=cwd).strip() != "false":
        raise ValueError("Full commit history is required")
    commits = git("rev-list", "--reverse", base + ".." + head, cwd=cwd).splitlines()
    if not commits:
        raise ValueError("No contribution commits found in the supplied range")
    failures = []
    for sha in commits:
        message = git("show", "-s", "--format=%B", sha, cwd=cwd)
        if not has_signoff(message):
            failures.append(sha)
    return commits, failures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("base_sha")
    parser.add_argument("head_sha")
    args = parser.parse_args()
    try:
        commits, failures = check_range(args.base_sha, args.head_sha)
    except (ValueError, UnicodeError, subprocess.CalledProcessError) as error:
        # Do not echo untrusted commit messages or identities into workflow logs.
        detail = str(error) if isinstance(error, ValueError) else "Cannot read commit history"
        print("DCO check failed: " + detail, file=sys.stderr)
        return 1
    for sha in failures:
        print(f"DCO check failed: {sha} lacks a valid final Signed-off-by trailer", file=sys.stderr)
    if failures:
        return 1
    print(f"DCO signoffs verified for all {len(commits)} contribution commits.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
