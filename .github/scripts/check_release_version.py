"""Check checked-in source and lockfile versions before starting native builds."""
from pathlib import Path
import sys
import tomllib


def check(version, root):
    manifests = [tomllib.loads((root / path).read_text())["package"] for path in ("Cargo.toml", "gui/Cargo.toml")]
    locked = tomllib.loads((root / "Cargo.lock").read_text())["package"]
    for package in manifests:
        versions = [p["version"] for p in locked if p["name"] == package["name"] and "source" not in p]
        if versions != [package["version"]] or version != "v" + package["version"]:
            raise ValueError("Requested tag must match both Cargo manifests and their locked versions")
    if not (root / "docs/releases" / (version + ".md")).is_file():
        raise ValueError("Release notes missing for the requested version")


if __name__ == "__main__":
    check(sys.argv[1], Path.cwd())
