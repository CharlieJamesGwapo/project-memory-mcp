#!/usr/bin/env python3
"""Create native release archives without losing Unix executable permissions."""
import argparse
import hashlib
from pathlib import Path
import re
import tarfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def build_archive(binary, target, output):
    binary = Path(binary).resolve(strict=True)
    output = Path(output)
    if not re.fullmatch(r"[a-zA-Z0-9_-]+", target):
        raise ValueError("Invalid target name")
    output.mkdir(parents=True, exist_ok=True)
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    name = f"project-memory-mcp-{version}-{target}"
    windows = "windows" in target
    archive = output / (name + (".zip" if windows else ".tar.gz"))
    assets = [(binary, name + "/project-memory-mcp" + (".exe" if windows else ""))]
    for filename in ("README.md", "LICENSE", "server.json", "SECURITY.md", "docs/skills.md", "docs/publishing.md"):
        assets.append((ROOT / filename, name + "/" + filename))
    for folder in ("skills", "examples"):
        for path in sorted((ROOT / folder).rglob("*")):
            if path.is_file():
                assets.append((path, name + "/" + path.relative_to(ROOT).as_posix()))
    if windows:
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as destination:
            for path, member in assets:
                destination.write(path, member)
    else:
        def portable(member):
            member.uid = member.gid = 0
            member.uname = member.gname = ""
            return member
        with tarfile.open(archive, "w:gz") as destination:
            for path, member in assets:
                destination.add(path, arcname=member, recursive=False, filter=portable)
    checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
    Path(str(archive) + ".sha256").write_text(f"{checksum}  {archive.name}\n")
    return archive


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output", type=Path, default=Path("artifacts"))
    args = parser.parse_args()
    filename = "project-memory-mcp.exe" if "windows" in args.target else "project-memory-mcp"
    binary = args.binary or ROOT / "target" / args.target / "release" / filename
    print(build_archive(binary, args.target, args.output))
