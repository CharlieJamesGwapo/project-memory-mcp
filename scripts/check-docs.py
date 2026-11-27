#!/usr/bin/env python3
"""Check local guide links, public MCP tool inventory and source version agreement."""
import argparse
import json
from pathlib import Path
import re
import tomllib
from urllib.parse import unquote, urlsplit


def check_docs(root):
    root = Path(root).resolve()
    errors = []
    guides = [root / name for name in ("README.md", "AGENTS.md", "CONTRIBUTING.md", "SECURITY.md", "CHANGELOG.md")]
    guides.extend((root / "docs").glob("*.md"))
    for guide in guides:
        if not guide.exists():
            continue
        if not guide.resolve().is_relative_to(root):
            errors.append(f"Guide outside checkout: {guide.name}")
            continue
        for match in re.finditer(r"!?\[[^\]\n]*\]\(([^\n]+?)\)", guide.read_text()):
            target = match.group(1).strip()
            if target.startswith("<") and ">" in target:
                target = target[1:target.index(">")]
            else:
                target = target.split()[0]
            parsed = urlsplit(target)
            if parsed.scheme or parsed.netloc or not parsed.path:
                continue
            destination = (guide.parent / unquote(parsed.path)).resolve()
            if not destination.is_relative_to(root):
                errors.append(f"{guide.relative_to(root)}: link outside checkout: {target}")
            elif not destination.exists():
                errors.append(f"{guide.relative_to(root)}: missing link target: {target}")
    try:
        source = (root / "src/server.rs").read_text()
        tools = set(re.findall(r"#\[tool\(.*?\)\]\s*async\s+fn\s+(\w+)", source, re.DOTALL))
        documented = set(re.findall(r"^\|\s*`([a-z][a-z0-9_]*)`\s*\|", (root / "docs/api-reference.md").read_text(), re.MULTILINE))
        if not tools or tools != documented:
            errors.append(f"API tool inventory mismatch: undocumented={sorted(tools-documented)}, unknown={sorted(documented-tools)}")
        version = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
        metadata = json.loads((root / "server.json").read_text())
        versions = {"server.json": metadata["version"], "server.json package": metadata["packages"][0]["version"]}
        for name in ("apps/studio/package.json", "apps/studio/src-tauri/tauri.conf.json"):
            versions[name] = json.loads((root / name).read_text())["version"]
        versions["apps/studio/src-tauri/Cargo.toml"] = tomllib.loads((root / "apps/studio/src-tauri/Cargo.toml").read_text())["package"]["version"]
        errors.extend(f"Version mismatch: {name}={value}, root={version}" for name, value in versions.items() if value != version)
    except (OSError, KeyError, ValueError, tomllib.TOMLDecodeError) as error:
        errors.append(f"Could not verify documentation metadata: {type(error).__name__}")
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    errors = check_docs(args.root)
    if errors:
        for error in errors:
            print(error)
        raise SystemExit(1)
    print("Guide link targets, MCP tool inventory and source versions verified (external URLs/anchors not fetched)")


if __name__ == "__main__":
    main()
