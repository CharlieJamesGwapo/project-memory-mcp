#!/usr/bin/env python3
"""Install the bundled skills offline; preview by default and preserve local edits."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import sys
import tempfile

SKILLS = ("project-memory", "resume-project", "record-decision", "prepare-handoff")
FORMAT_VERSION = 1


def absolute(path):
    # Do not resolve links before inspecting them.
    return Path(os.path.abspath(os.path.expanduser(str(path))))


def no_links(path):
    for entry in (path, *path.parents):
        if entry.is_symlink():
            raise ValueError(f"Symlink paths are not supported: {entry}")
        if entry.exists() and not entry.is_dir() and entry != path:
            raise ValueError(f"Parent is not a directory: {entry}")


def inventory(folder):
    """Validate every entry and hash paths/content without following links."""
    no_links(folder)
    if not folder.is_dir():
        raise ValueError(f"Expected skill directory: {folder}")
    digest = hashlib.sha256()
    for entry in sorted(folder.rglob("*")):
        mode = entry.lstat().st_mode
        relative = entry.relative_to(folder).as_posix().encode("utf-8")
        if stat.S_ISLNK(mode) or not (stat.S_ISREG(mode) or stat.S_ISDIR(mode)):
            raise ValueError(f"Only regular files/directories are supported: {entry}")
        digest.update(b"d" if stat.S_ISDIR(mode) else b"f")
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        if stat.S_ISREG(mode):
            data = entry.read_bytes()
            digest.update(len(data).to_bytes(8, "big"))
            digest.update(data)
    return digest.hexdigest()


def source_inventory(source):
    no_links(source)
    result = {}
    for name in SKILLS:
        folder = source / name
        result[name] = inventory(folder)
        if not (folder / "SKILL.md").is_file():
            raise ValueError(f"Missing SKILL.md: {folder}")
    return result


def plan_install(source, destination, replace):
    no_links(destination)
    try:
        destination.relative_to(source)
    except ValueError:
        pass
    else:
        raise ValueError("Destination must be outside the bundled skills source")
    if destination.exists() and not destination.is_dir():
        raise ValueError(f"Destination is not a directory: {destination}")
    expected = source_inventory(source)
    actions = {}
    previous = {}
    for name in SKILLS:
        target = destination / name
        if target.exists() or target.is_symlink():
            previous[name] = inventory(target)
            if previous[name] == expected[name]:
                actions[name] = "unchanged"
            elif replace:
                actions[name] = "replace"
            else:
                raise ValueError(f"Existing skill differs: {target}; preserve it or preview with --replace")
        else:
            actions[name] = "install"
    return actions, expected, previous


def print_plan(actions, apply, operation="install"):
    print(f"{'Apply' if apply else 'Dry-run'} {operation} plan:")
    for name, action in actions.items():
        print(f"  {name}: {action}")
    if not apply:
        print("No files changed. Add --apply to perform this plan.")


def apply_install(source, destination, actions, expected, previous):
    changed = {name: action for name, action in actions.items() if action != "unchanged"}
    if not changed:
        return None
    destination.mkdir(parents=True, exist_ok=True)
    no_links(destination)
    lock = destination / ".project-memory-install.lock"
    lock.mkdir()  # Exclusive per-destination operation; an existing lock is never removed here.
    staging = None
    backup = None
    moved = []
    installed = []
    try:
        staging = Path(tempfile.mkdtemp(prefix=".project-memory-stage-", dir=str(destination)))
        for name in changed:
            shutil.copytree(source / name, staging / name)
            if inventory(staging / name) != expected[name]:
                raise ValueError(f"Source changed while staging: {name}; retry after checking the source")
        # Check the destination again after staging; do not replace intervening edits.
        for name, action in changed.items():
            target = destination / name
            if action == "install":
                if target.exists() or target.is_symlink():
                    raise ValueError(f"Destination changed while staging: {target}")
            elif inventory(target) != previous[name]:
                raise ValueError(f"Destination changed while staging: {target}")
        if "replace" in changed.values():
            stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
            backup = Path(tempfile.mkdtemp(prefix=f".project-memory-backup-{stamp}-", dir=str(destination)))
            manifest = {
                "format_version": FORMAT_VERSION,
                "destination": str(destination),
                "actions": changed,
                "installed_hashes": {name: expected[name] for name in changed},
                "previous_hashes": {name: previous[name] for name, action in changed.items() if action == "replace"},
            }
            (backup / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        for name, action in changed.items():
            target = destination / name
            if action == "replace":
                target.rename(backup / name)
                moved.append(name)
            (staging / name).rename(target)
            installed.append(name)
    except BaseException:
        # Ordinary errors and interruptions roll back the folders already changed.
        # If recovery itself fails, retain the backup for explicit user recovery.
        for name in reversed(installed):
            shutil.rmtree(destination / name)
        for name in reversed(moved):
            (backup / name).rename(destination / name)
        if backup is not None:
            shutil.rmtree(backup)
        raise
    finally:
        if staging is not None:
            shutil.rmtree(staging)
        lock.rmdir()
    return backup


def plan_restore(backup, destination):
    no_links(destination)
    no_links(backup)
    if backup.parent != destination or not backup.name.startswith(".project-memory-backup-"):
        raise ValueError("Restore requires an installer backup directly inside this destination")
    inventory(backup)
    manifest = json.loads((backup / "manifest.json").read_text(encoding="utf-8"))
    if manifest.get("format_version") != FORMAT_VERSION or manifest.get("destination") != str(destination):
        raise ValueError("Backup format/destination does not match")
    actions = manifest.get("actions")
    installed = manifest.get("installed_hashes")
    previous = manifest.get("previous_hashes")
    if not isinstance(actions, dict) or not actions or not isinstance(installed, dict) or not isinstance(previous, dict):
        raise ValueError("Invalid backup manifest")
    if any(name not in SKILLS or action not in ("install", "replace") for name, action in actions.items()):
        raise ValueError("Invalid backup skill/action")
    replacements = {name for name, action in actions.items() if action == "replace"}
    if set(installed) != set(actions) or set(previous) != replacements:
        raise ValueError("Invalid backup hash inventory")
    for name, action in actions.items():
        if inventory(destination / name) != installed[name]:
            raise ValueError(f"Installed skill changed after upgrade: {name}; preserve your edits before restoring")
        if action == "replace" and inventory(backup / name) != previous[name]:
            raise ValueError(f"Backup skill changed: {name}")
    return actions


def apply_restore(backup, destination, actions):
    lock = destination / ".project-memory-install.lock"
    lock.mkdir()
    staging = None
    moved = []
    restored = []
    try:
        staging = Path(tempfile.mkdtemp(prefix=".project-memory-stage-", dir=str(destination)))
        for name, action in actions.items():
            if action == "replace":
                shutil.copytree(backup / name, staging / f"old-{name}")
        # Revalidate all hashes immediately before changing any target.
        plan_restore(backup, destination)
        for name, action in actions.items():
            (destination / name).rename(staging / f"new-{name}")
            moved.append(name)
            if action == "replace":
                (staging / f"old-{name}").rename(destination / name)
                restored.append(name)
    except BaseException:
        for name in reversed(restored):
            shutil.rmtree(destination / name)
        for name in reversed(moved):
            (staging / f"new-{name}").rename(destination / name)
        raise
    finally:
        if staging is not None:
            shutil.rmtree(staging)
        lock.rmdir()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--destination", required=True, help="Explicit skill directory; use your host's documented location")
    parser.add_argument("--apply", action="store_true", help="Apply the previewed operation (default is dry-run)")
    parser.add_argument("--replace", action="store_true", help="Replace different existing skill folders, preserving a backup")
    parser.add_argument("--restore", metavar="BACKUP", help="Restore an installer backup; previews unless --apply is supplied")
    args = parser.parse_args(argv)
    if args.restore and args.replace:
        parser.error("--restore and --replace cannot be combined")
    destination = absolute(args.destination)
    try:
        if args.restore:
            backup = absolute(args.restore)
            actions = plan_restore(backup, destination)
            print_plan({name: "restore previous" if action == "replace" else "remove newly installed" for name, action in actions.items()}, args.apply, "restore")
            if args.apply:
                apply_restore(backup, destination, actions)
                print(f"Previous skills restored; backup retained at {backup}")
        else:
            source = Path(__file__).resolve().parents[1] / "skills"
            actions, expected, previous = plan_install(source, destination, args.replace)
            print_plan(actions, args.apply)
            if args.apply:
                backup = apply_install(source, destination, actions, expected, previous)
                print("Skill pack installed. No host configuration or AGENTS.md was changed.")
                if backup is not None:
                    print(f"Backup: {backup}")
        return 0
    except (OSError, ValueError, TypeError, KeyError) as error:
        print(f"Skill operation refused: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
