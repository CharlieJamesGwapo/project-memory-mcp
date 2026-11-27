#!/usr/bin/env python3
"""Check an actual Cargo archive for required assets and accidental private data."""
import json
import sys
import tarfile
import tomllib
from pathlib import PurePosixPath


def check(path):
    with tarfile.open(path, "r:gz") as archive:
        members = archive.getmembers()
        roots = {PurePosixPath(m.name).parts[0] for m in members}
        if len(roots) != 1:
            raise ValueError("Expected one package root")
        files = {}
        for member in members:
            parts = PurePosixPath(member.name).parts
            if member.issym() or member.islnk() or ".." in parts:
                raise ValueError(f"Unexpected archive link/path: {member.name}")
            name = "/".join(parts[1:])
            if member.isfile():
                files[name] = member
            forbidden = {"target", ".git", "node_modules", ".env", "credentials", "work"}
            if forbidden.intersection(parts[1:]) or any(
                part.endswith((".sqlite", ".sqlite-wal", ".sqlite-shm", ".db", ".log"))
                or part.startswith(".env.") for part in parts[1:]
            ):
                raise ValueError(f"Private/generated asset in package: {name}")
        required = {"Cargo.toml", "Cargo.lock", "src/main.rs", "src/lib.rs", "README.md", "LICENSE", "server.json", "docs/publishing.md", "docs/skills.md"}
        required.update(f"skills/{name}/SKILL.md" for name in ("project-memory", "resume-project", "record-decision", "prepare-handoff"))
        required.update(f"skills/project-memory/references/{name}.md" for name in ("token-saver", "code-review", "agent-team", "security-audit"))
        required.update({"AGENTS.md", "CHANGELOG.md", "docs/quickstart.md", "docs/architecture.md", "docs/api-reference.md", "docs/security.md", "docs/data-safety.md", "scripts/install-skills.py", "scripts/check-skills.py", "scripts/backup-memory.py", "scripts/verify-backup.py", "docs/access-policy.md", "docs/code-review.md", "docs/backup-verification.md", "docs/setup.md", "docs/memory-health.md", "docs/mcp-views.md"})
        missing = required.difference(files)
        if missing:
            raise ValueError(f"Missing assets: {sorted(missing)}")
        def read(name):
            return archive.extractfile(files[name]).read().decode("utf-8")
        cargo = tomllib.loads(read("Cargo.toml"))
        metadata = json.loads(read("server.json"))
        package = cargo["package"]
        if metadata["version"] != package["version"] or metadata["packages"][0]["identifier"] != package["name"]:
            raise ValueError("Cargo and MCP package identities differ")
        if metadata["packages"][0]["version"] != package["version"]:
            raise ValueError("Cargo and MCP package versions differ")
        token = f'mcp-name: {metadata["name"]}'
        if token not in read("README.md"):
            raise ValueError("Missing visible Cargo ownership token")
        print(f"Package verified: {len(files)} files; source, skills, identity and private-data exclusions pass")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Usage: python3 scripts/check-package.py path/to/package.crate (Python >=3.11)")
    try:
        check(sys.argv[1])
    except (ValueError, OSError, KeyError, tarfile.TarError) as exc:
        raise SystemExit(str(exc)) from exc
