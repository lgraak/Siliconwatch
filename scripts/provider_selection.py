"""Read-only registration evidence and owned, temporary class-root aliases.

Private manifests contain paths/inodes for this session only. Tokens are not
persistent sensor identities. Only public_inventory() is suitable for export.
Aliases confer no extra permissions: the unchanged reader only reads through
them; this module only writes/unlinks its own local symlink and directory.
"""

from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import stat
import tempfile


MAX_REGISTRATIONS = 256
MAX_DIRECTORY_ENTRIES = 4096
_ENTRY = re.compile(r"hwmon[0-9]+\Z")
_PROVIDER = re.compile(r"[A-Za-z0-9_.+-]{1,80}\Z")


class SelectionError(ValueError):
    """Safe diagnostic code, without source paths or exception text."""


def _error(error):
    if isinstance(error, PermissionError):
        return "permission_denied"
    if isinstance(error, FileNotFoundError):
        return "missing"
    return "io_error"


def _names(directory):
    names = []
    with os.scandir(directory) as entries:
        for entry in entries:
            if len(names) >= MAX_DIRECTORY_ENTRIES:
                raise SelectionError("directory_entry_limit")
            names.append(entry.name)
    return sorted(names)


def _provider(path):
    try:
        with path.open("r", encoding="utf-8", errors="strict") as stream:
            name = stream.read(257)
        if len(name) > 256 or not _PROVIDER.fullmatch(name.strip()):
            return None, "provider_invalid"
        return name.strip(), None
    except (OSError, UnicodeError) as error:
        return None, "provider_" + (_error(error) if isinstance(error, OSError) else "invalid")


def _inspect(path, root, root_target, root_identity):
    item = {"entry": path.name, "path": str(path), "root": str(root),
            "root_target": str(root_target), "root_identity": root_identity,
            "target": None, "target_identity": None, "provider": None,
            "channels": [], "status": "unavailable", "issues": [],
            "device_link": None, "device_link_status": "missing"}
    try:
        target = path.resolve(strict=True)
        metadata = target.stat()
        if not stat.S_ISDIR(metadata.st_mode):
            item["issues"].append("registration_not_directory")
            return item
        item.update(target=str(target), target_identity=[metadata.st_dev, metadata.st_ino])
        provider, problem = _provider(path / "name")
        item["provider"] = provider
        if problem:
            item["issues"].append(problem)
        item["channels"] = [name for name in _names(path) if name.endswith("_input")]
        link = path / "device"
        try:
            link_meta = link.lstat()
            if not stat.S_ISLNK(link_meta.st_mode):
                item["device_link_status"] = "not_symlink"
            else:
                linked = link.resolve(strict=True)
                if linked.is_dir():
                    item["device_link"] = str(linked)
                    item["device_link_status"] = "ok"
                else:
                    item["device_link_status"] = "not_directory"
        except (OSError, RuntimeError) as error:
            item["device_link_status"] = _error(error) if isinstance(error, OSError) else "symlink_loop"
        # Identity evidence failure does not invent a reading failure. The Rust
        # reader remains responsible for identity status and collision context.
        item["status"] = "unavailable" if problem else ("ok" if item["channels"] else "zero_inputs")
    except SelectionError:
        raise
    except (OSError, RuntimeError) as error:
        item["issues"].append("registration_" + (_error(error) if isinstance(error, OSError) else "symlink_loop"))
    return item


def enumerate_registrations(root):
    """Return bounded private evidence; unavailable registrations remain visible."""
    root = Path(root).absolute()
    try:
        root_target = root.resolve(strict=True)
        metadata = root_target.stat()
        if not stat.S_ISDIR(metadata.st_mode):
            raise SelectionError("root_not_directory")
        names = [name for name in _names(root) if _ENTRY.fullmatch(name)]
        if len(names) > MAX_REGISTRATIONS:
            raise SelectionError("registration_limit")
    except (OSError, RuntimeError) as error:
        raise SelectionError("root_" + (_error(error) if isinstance(error, OSError) else "symlink_loop")) from None
    result = [_inspect(root / name, root, root_target, [metadata.st_dev, metadata.st_ino])
              for name in names]
    # Lexical class-entry order is deterministic within this manifest. These
    # tokens are intentionally invalidated by renumbering, never durable IDs.
    targets = {}
    for item in result:
        if item["target"] is not None:
            targets.setdefault(item["target"], []).append(item)
    for aliases in targets.values():
        if len(aliases) > 1:
            for item in aliases:
                item["status"] = "duplicate_target"
                item["issues"].append("duplicate_canonical_registration")
    for index, item in enumerate(result, 1):
        item["token"] = f"r{index:03d}"
    return result


def public_inventory(registrations):
    """Allowlisted result metadata; never export identity paths/inodes."""
    return [{"token": item["token"], "provider": item["provider"],
             "channel_count": len(item["channels"]), "status": item["status"],
             "issues": list(item["issues"]), "device_link_status": item["device_link_status"]}
            for item in registrations]


def fingerprint(registrations):
    """Private deterministic comparison covering renumbering and target changes."""
    return json.dumps(registrations, sort_keys=True, separators=(",", ":"), allow_nan=False)


def select_registrations(registrations, selectors=None, max_count=5):
    if not isinstance(max_count, int) or isinstance(max_count, bool) or not 1 <= max_count <= 5:
        raise SelectionError("selection_limit_must_be_1_to_5")
    if selectors is None:
        selected = [item for item in registrations if item["status"] == "ok"]
    else:
        if isinstance(selectors, str):
            raise SelectionError("selectors_must_be_a_sequence")
        selected = []
        for selector in selectors:
            if not isinstance(selector, str) or not selector:
                raise SelectionError("invalid_selection")
            matches = [item for item in registrations if item["token"] == selector]
            if not matches:
                matches = [item for item in registrations if item["provider"] == selector]
            if not matches:
                raise SelectionError("unknown_selection")
            if len(matches) != 1:
                raise SelectionError("ambiguous_selection")
            item = matches[0]
            if item["status"] != "ok":
                raise SelectionError("unavailable_selection")
            if any(item["token"] == previous["token"] or item["target"] == previous["target"]
                   for previous in selected):
                raise SelectionError("duplicate_selection")
            selected.append(item)
    if not selected:
        raise SelectionError("no_usable_registrations")
    if len(selected) > max_count:
        raise SelectionError("too_many_selections_choose_at_most_five")
    # Caller input order cannot silently change experiment ordering.
    return sorted(selected, key=lambda item: item["token"])


def validate_registration(registration):
    """Read-only validation immediately before/after using an alias root."""
    current = enumerate_registrations(registration["root"])
    match = next((item for item in current if item["entry"] == registration["entry"]), None)
    if match is None or fingerprint([match]) != fingerprint([registration]):
        raise SelectionError("registration_changed")
    if match["status"] != "ok":
        raise SelectionError("unavailable_selection")


def _private_parent(parent, registration):
    parent = Path(parent).absolute()
    try:
        resolved = parent.resolve(strict=True)
        metadata = parent.lstat()
        if resolved != parent or not stat.S_ISDIR(metadata.st_mode):
            raise SelectionError("alias_parent_not_plain_directory")
        if metadata.st_uid != os.getuid() or stat.S_IMODE(metadata.st_mode) & 0o077:
            raise SelectionError("alias_parent_not_private")
        protected = [Path("/sys"), Path("/proc"), Path("/dev"),
                     Path(registration["root_target"]), Path(registration["target"])]
        if registration["device_link"] is not None:
            protected.append(Path(registration["device_link"]))
        target = Path(registration["target"])
        protected.append(target.parent.parent if target.parent.name == "hwmon" else target.parent)
        if any(parent == path or parent.is_relative_to(path) for path in protected):
            raise SelectionError("alias_parent_inside_source_tree")
    except (OSError, RuntimeError):
        raise SelectionError("alias_parent_unavailable") from None
    return parent


@contextmanager
def alias_root(parent, registration):
    """Yield one owned root and remove only its symlink/empty directory.

    Descriptor-relative cleanup never follows a substituted root/link and never
    recursively deletes any directory. Unexpected contents remain for diagnosis.
    """
    validate_registration(registration)
    parent = _private_parent(parent, registration)
    parent_fd = os.open(parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    root = None
    root_fd = None
    try:
        root = Path(tempfile.mkdtemp(prefix="selected-", dir=parent))
        root_fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        os.symlink(registration["target"], registration["entry"], dir_fd=root_fd)
        try:
            yield root
        finally:
            validate_registration(registration)
    finally:
        try:
            if root_fd is not None:
                # The directory was opened before inserting its alias. Rewind
                # the held stream explicitly before inspecting current entries;
                # a descriptor may otherwise expose a stale enumeration view.
                os.lseek(root_fd, 0, os.SEEK_SET)
                names = os.listdir(root_fd)
                if names not in ([], [registration["entry"]]):
                    raise SelectionError("alias_cleanup_unexpected_entries")
                if names:
                    metadata = os.stat(registration["entry"], dir_fd=root_fd, follow_symlinks=False)
                    if not stat.S_ISLNK(metadata.st_mode):
                        raise SelectionError("alias_cleanup_not_symlink")
                    os.unlink(registration["entry"], dir_fd=root_fd)
            if root is not None:
                os.rmdir(root.name, dir_fd=parent_fd)
        except OSError:
            raise SelectionError("alias_cleanup_failed") from None
        finally:
            if root_fd is not None:
                os.close(root_fd)
            os.close(parent_fd)
