#!/usr/bin/env python3
"""Check required single-line frontmatter and local skill links, without YAML execution."""
import argparse
import json
from pathlib import Path
import re
import stat
import sys
from urllib.parse import unquote, urlsplit

SKILLS = ("project-memory", "resume-project", "record-decision", "prepare-handoff")
LINK = re.compile(r"\[[^\]]*\]\(([^\s)]+)(?:\s+\"[^\"]*\")?\)")


def scalar(value):
    if not value or value[0] in "!&*{|>[":
        raise ValueError("Required frontmatter values must be plain or quoted single-line text")
    if value.startswith('"'):
        value = json.loads(value)
    elif value.startswith("'"):
        if len(value) < 2 or not value.endswith("'"):
            raise ValueError("Unclosed quoted frontmatter value")
        value = value[1:-1].replace("''", "'")
    if not isinstance(value, str) or not value.strip():
        raise ValueError("Required frontmatter text must not be empty")
    return value


def inspect_skill(path):
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    if not lines or lines[0] != "---":
        raise ValueError(f"Missing frontmatter: {path}")
    try:
        closing = lines.index("---", 1)
    except ValueError:
        raise ValueError(f"Unclosed frontmatter: {path}") from None
    required = {}
    for line in lines[1:closing]:
        match = re.match(r"^(name|description):\s*(.*)$", line)
        if match:
            key, value = match.groups()
            if key in required:
                raise ValueError(f"Duplicate {key}: {path}")
            required[key] = scalar(value)
    if set(required) != {"name", "description"}:
        raise ValueError(f"Missing required name/description: {path}")
    name = required["name"]
    if name != path.parent.name or not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", name) or len(name) > 64:
        raise ValueError(f"Invalid name/folder match: {path}")
    if len(required["description"]) > 1024:
        raise ValueError(f"Description exceeds 1024 characters: {path}")
    if not any(line.strip() for line in lines[closing + 1:]):
        raise ValueError(f"Empty skill body: {path}")
    return text


def check(root):
    root = root.absolute()
    for entry in (root, *root.parents):
        if entry.is_symlink():
            raise ValueError(f"Symlink skill path: {entry}")
    markdown = []
    for name in SKILLS:
        folder = root / name
        if not folder.is_dir() or folder.is_symlink():
            raise ValueError(f"Missing regular skill folder: {folder}")
        for entry in folder.rglob("*"):
            mode = entry.lstat().st_mode
            if not (stat.S_ISREG(mode) or stat.S_ISDIR(mode)):
                raise ValueError(f"Non-regular skill asset: {entry}")
            if entry.is_file() and entry.suffix == ".md":
                markdown.append(entry)
        inspect_skill(folder / "SKILL.md")
    for path in markdown:
        for match in LINK.finditer(path.read_text(encoding="utf-8")):
            target = urlsplit(match.group(1))
            if target.scheme in ("https", "http", "mailto"):
                continue  # Never fetch external links during this offline check.
            if target.scheme or target.netloc:
                raise ValueError(f"Unsupported link scheme in {path}: {match.group(1)}")
            if not target.path:
                continue
            relative = Path(unquote(target.path))
            if relative.is_absolute():
                raise ValueError(f"Absolute local reference in {path}: {relative}")
            linked = (path.parent / relative).resolve()
            try:
                linked.relative_to(root.resolve())
            except ValueError:
                raise ValueError(f"Reference escapes installed pack: {path}: {relative}") from None
            if not linked.is_file():
                raise ValueError(f"Missing local reference: {path}: {relative}")
    print(f"Skills checked: {len(SKILLS)} skills, {len(markdown)} Markdown files; required frontmatter and local references pass")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skills-root", type=Path, default=Path(__file__).resolve().parents[1] / "skills")
    args = parser.parse_args(argv)
    try:
        check(args.skills_root)
    except (OSError, UnicodeError, ValueError, TypeError) as error:
        print(f"Skill check failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
