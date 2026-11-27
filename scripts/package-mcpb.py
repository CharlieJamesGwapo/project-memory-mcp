#!/usr/bin/env python3
"""Stage native Rust binaries and skills for an official MCPB pack operation."""
import argparse
import json
from pathlib import Path
import shutil
import tomllib

ROOT = Path(__file__).resolve().parents[1]

TOOLS = [
    ("create_project", "Create a project with a stable UUID."),
    ("list_projects", "List saved projects with pagination."),
    ("get_project_context", "Read bounded project context and next actions."),
    ("get_record", "Retrieve a complete project record."),
    ("save_note", "Save an explicit project note with provenance."),
    ("search_notes", "Search literal note substrings in one project."),
    ("record_decision", "Preserve decisions and explicit supersession history."),
    ("save_handoff", "Save progress, actual verification, and next actions."),
    ("export_project", "Export paginated project records as Markdown."),
    ("delete_project", "Delete one project only with explicit confirmation."),
]


def stage_bundle(macos_universal, linux_x64, windows_x64, destination):
    inputs = [Path(path).resolve(strict=True) for path in (macos_universal, linux_x64, windows_x64)]
    destination = Path(destination)
    destination.mkdir(parents=True, exist_ok=False)
    for source, relative in zip(inputs, (
        "server/darwin/project-memory-mcp",
        "server/linux/project-memory-mcp",
        "server/win32/project-memory-mcp.exe",
    )):
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        target.chmod(0o755)
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    manifest = {
        "manifest_version": "0.3",
        "name": "project-memory-mcp",
        "display_name": "Project Memory",
        "version": version,
        "description": "Local project notes, decision history, and handoffs for AI assistants.",
        "long_description": "A native Rust MCP server with local SQLite storage, ten tools, and three companion skills. No paid API or hosted service is required. Binary platforms: macOS Apple Silicon and Intel (universal binary), Linux x64 (requires a compatible glibc), and Windows x64. Saved data stays in the platform user data directory; the AI client controls what is sent to its model. Skills are included as files for manual installation in a compatible client. Binaries are unsigned.",
        "author": {"name": "CharlieJamesGwapo", "url": "https://github.com/CharlieJamesGwapo"},
        "repository": {"type": "git", "url": "https://github.com/CharlieJamesGwapo/project-memory-mcp"},
        "homepage": "https://github.com/CharlieJamesGwapo/project-memory-mcp",
        "license": "MIT",
        "keywords": ["project", "memory", "rust", "sqlite", "handoff"],
        "server": {
            "type": "binary",
            "entry_point": "server/darwin/project-memory-mcp",
            "mcp_config": {
                "command": "${__dirname}/server/darwin/project-memory-mcp",
                "args": [],
                "platform_overrides": {
                    "linux": {"command": "${__dirname}/server/linux/project-memory-mcp"},
                    "win32": {"command": "${__dirname}/server/win32/project-memory-mcp.exe"},
                },
            },
        },
        "compatibility": {"platforms": ["darwin", "linux", "win32"]},
        "tools": [{"name": name, "description": description} for name, description in TOOLS],
    }
    (destination / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    for name in ("README.md", "LICENSE", "SECURITY.md", "docs/skills.md", "docs/publishing.md", "examples/mcp-config.json"):
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / name, target)
    shutil.copytree(ROOT / "skills", destination / "skills")
    return destination


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--macos-universal", type=Path, required=True)
    parser.add_argument("--linux-x64", type=Path, required=True)
    parser.add_argument("--windows-x64", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    print(stage_bundle(args.macos_universal, args.linux_x64, args.windows_x64, args.output_dir))
