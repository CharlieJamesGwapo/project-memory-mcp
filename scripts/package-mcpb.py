#!/usr/bin/env python3
"""Stage, validate and ZIP our generated native MCPB using only Python's stdlib.

MCPB is a ZIP with root manifest.json; the CLI is optional:
https://github.com/modelcontextprotocol/mcpb/blob/main/README.md
https://github.com/modelcontextprotocol/mcpb/blob/main/MANIFEST.md
This validates this project's exact bundle contract, not arbitrary MCPB manifests.
"""
import argparse
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SKILLS = ('project-memory', 'resume-project', 'record-decision', 'prepare-handoff')
BINARIES = ('server/darwin/project-memory-mcp', 'server/linux/project-memory-mcp',
            'server/win32/project-memory-mcp.exe')
REQUIRED_ASSETS = ('README.md', 'LICENSE', 'SECURITY.md', 'docs/skills.md',
                   'docs/publishing.md', 'examples/mcp-config.json',
                   'scripts/install-skills.py', 'scripts/check-skills.py',
                   'scripts/backup-memory.py', 'scripts/verify-backup.py')
OPTIONAL_ASSETS = ('CHANGELOG.md', 'CONTRIBUTING.md', 'AGENTS.md', *(f'docs/{name}.md' for name in (
    'quickstart', 'architecture', 'api-reference', 'workflows', 'troubleshooting',
    'data-safety', 'security', 'studio', 'cli', 'skill-evaluation', 'retrieval-evaluation', 'access-policy', 'code-review', 'backup-verification', 'setup', 'memory-health', 'mcp-views')))
MAX_MEMBER_BYTES = 128 * 1024 * 1024
MAX_TOTAL_BYTES = 384 * 1024 * 1024
TOOLS = [
    ('create_project', 'Create a project with a stable UUID.'),
    ('list_projects', 'List saved projects with pagination.'),
    ('get_project_context', 'Read bounded project context and next actions.'),
    ('get_record', 'Retrieve a complete project record.'),
    ('save_note', 'Save an explicit project note with provenance.'),
    ('search_notes', 'Search literal note substrings in one project.'),
    ('record_decision', 'Preserve decisions and explicit supersession history.'),
    ('save_handoff', 'Save progress, actual verification, and next actions.'),
    ('export_project', 'Export paginated project records as Markdown.'),
    ('delete_project', 'Optional: deletion requires local policy opt-in and actual user authorization.'),
    ('search_memory', 'Search literal text across project record kinds.'),
    ('get_context_bundle', 'Retrieve whole records within an explicit byte budget.'),
    ('get_access_policy', 'Inspect effective local operation and project restrictions.'),
    ('save_review', 'Append a commit-linked review with findings and actual checks.'),
    ('get_review_history', 'Read complete review revisions and supplied-head freshness.'),
    ('get_memory_health', 'Inspect bounded historical metadata and review suggestions without changing records.'),
]


def regular_file(path):
    path = Path(path)
    if path.is_symlink() or not path.is_file():
        raise ValueError('Bundle inputs must be regular files, never links')
    if path.stat().st_size > MAX_MEMBER_BYTES:
        raise ValueError('Bundle member exceeds the size limit')
    return path


def tree_files(directory):
    directory = Path(directory)
    if directory.is_symlink() or not directory.is_dir():
        raise ValueError('Bundle directories must be real directories, never links')
    files = {}
    directories = set()
    for base, dirs, names in os.walk(directory, followlinks=False):
        for name in dirs:
            path = Path(base) / name
            if path.is_symlink():
                raise ValueError('Bundle directories cannot contain links')
            directories.add(path.relative_to(directory).as_posix())
        for name in names:
            path = regular_file(Path(base) / name)
            files[path.relative_to(directory).as_posix()] = path
    return files, directories


def asset_sources():
    assets = {name: regular_file(ROOT / name) for name in REQUIRED_ASSETS}
    for name in OPTIONAL_ASSETS:
        if (ROOT / name).exists() or (ROOT / name).is_symlink():
            assets[name] = regular_file(ROOT / name)
    for name in SKILLS:
        directory = ROOT / 'skills' / name
        files, _ = tree_files(directory)
        if 'SKILL.md' not in files:
            raise ValueError('Every bundled skill requires SKILL.md')
        for relative, path in files.items():
            if path.suffix != '.md' and (name, relative) != ('project-memory', 'agents/openai.yaml'):
                raise ValueError('Only Markdown and the reviewed router UI metadata are permitted inside bundled skills')
            assets[f'skills/{name}/{relative}'] = path
    return assets


def manifest():
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    if not re.fullmatch(r'\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?(?:\+[A-Za-z0-9.-]+)?', version):
        raise ValueError('Invalid Cargo package version')
    return {
        'manifest_version': '0.3', 'name': 'project-memory-mcp',
        'display_name': 'Project Memory', 'version': version,
        'description': 'Local project notes, decisions, handoffs, and bounded context for AI assistants.',
        'long_description': f'A native Rust MCP server with local SQLite storage, {len(TOOLS)} tools, and {len(SKILLS)} companion skills. No paid API or hosted service is required. Binary platforms: macOS Apple Silicon and Intel (universal binary), Linux x64 (requires a compatible glibc), and Windows x64. Saved data stays in the platform user data directory; the AI client controls what is sent to its model. Skills and the local installer are included for a compatible client. Binaries are unsigned.',
        'author': {'name': 'CharlieJamesGwapo', 'url': 'https://github.com/CharlieJamesGwapo'},
        'repository': {'type': 'git', 'url': 'https://github.com/CharlieJamesGwapo/project-memory-mcp'},
        'homepage': 'https://github.com/CharlieJamesGwapo/project-memory-mcp',
        'license': 'MIT', 'keywords': ['project', 'memory', 'rust', 'sqlite', 'handoff'],
        'server': {'type': 'binary', 'entry_point': BINARIES[0], 'mcp_config': {
            'command': '${__dirname}/' + BINARIES[0], 'args': [], 'platform_overrides': {
                'linux': {'command': '${__dirname}/' + BINARIES[1]},
                'win32': {'command': '${__dirname}/' + BINARIES[2]}}}},
        'compatibility': {'platforms': ['darwin', 'linux', 'win32']},
        'tools': [{'name': name, 'description': description} for name, description in TOOLS],
    }


def validate_manifest(raw):
    if len(raw) > 256 * 1024 or json.loads(raw) != manifest():
        raise ValueError('Manifest does not match the current generated bundle contract')


def validate_directory(directory):
    files, directories = tree_files(directory)
    assets = asset_sources()
    expected = {'manifest.json', *BINARIES, *assets}
    expected_dirs = {parent.as_posix() for name in expected
                     for parent in PurePosixPath(name).parents if parent != PurePosixPath('.')}
    if set(files) != expected or not directories <= expected_dirs:
        raise ValueError('Bundle inventory contains missing or unexpected payloads')
    if sum(path.stat().st_size for path in files.values()) > MAX_TOTAL_BYTES:
        raise ValueError('Bundle exceeds the total size limit')
    validate_manifest(files['manifest.json'].read_bytes())
    for name, source in assets.items():
        if files[name].read_bytes() != source.read_bytes():
            raise ValueError('Bundled documentation or skill differs from the source checkout')
    return files


def stage_bundle(macos_universal, linux_x64, windows_x64, destination):
    inputs = [regular_file(Path(path).absolute()) for path in (macos_universal, linux_x64, windows_x64)]
    assets = asset_sources()
    destination = Path(destination)
    if destination.exists() or destination.is_symlink():
        raise FileExistsError(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.mcpb-stage-', dir=destination.parent) as temporary:
        staging = Path(temporary) / 'payload'
        staging.mkdir()
        for source, name in [*zip(inputs, BINARIES), *((path, name) for name, path in assets.items())]:
            target = staging / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
            target.chmod(0o755 if name in BINARIES else 0o644)
        (staging / 'manifest.json').write_text(json.dumps(manifest(), indent=2) + '\n')
        validate_directory(staging)
        staging.rename(destination)
    return destination


def pack_bundle(directory, archive):
    files = validate_directory(directory)
    archive = Path(archive)
    archive.parent.mkdir(parents=True, exist_ok=True)
    # Exclusive creation protects any prior release archive from replacement.
    # On I/O failure remove only the archive created by this operation.
    created = False
    try:
        with archive.open('xb') as raw:
            created = True
            with zipfile.ZipFile(raw, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as zipped:
                for name, source in sorted(files.items()):
                    member = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
                    member.create_system = 3
                    member.compress_type = zipfile.ZIP_DEFLATED
                    member.external_attr = (stat.S_IFREG | (0o755 if name in BINARIES else 0o644)) << 16
                    with source.open('rb') as incoming, zipped.open(member, 'w') as outgoing:
                        shutil.copyfileobj(incoming, outgoing)
    except BaseException:
        if created:
            archive.unlink(missing_ok=True)
        raise
    return archive


def unpack_bundle(archive, destination):
    assets = asset_sources()
    expected = {'manifest.json', *BINARIES, *assets}
    destination = Path(destination)
    if destination.exists() or destination.is_symlink():
        raise FileExistsError(destination)
    with zipfile.ZipFile(regular_file(archive)) as zipped:
        members = zipped.infolist()
        names = [member.filename for member in members]
        if len(names) != len(set(names)) or set(names) != expected:
            raise ValueError('Archive inventory contains missing, duplicate or unexpected payloads')
        for member in members:
            path = PurePosixPath(member.filename)
            kind = stat.S_IFMT(member.external_attr >> 16)
            if (path.is_absolute() or '..' in path.parts or '\\' in member.filename
                    or member.is_dir() or kind not in (0, stat.S_IFREG)
                    or member.flag_bits & 1 or member.file_size > MAX_MEMBER_BYTES):
                raise ValueError('Archive contains an unsafe member')
        if sum(member.file_size for member in members) > MAX_TOTAL_BYTES:
            raise ValueError('Archive exceeds the total size limit')
        validate_manifest(zipped.read('manifest.json'))
        for name, source in assets.items():
            if zipped.read(name) != source.read_bytes():
                raise ValueError('Archive documentation or skills differ from the source checkout')
        destination.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='.mcpb-unpack-', dir=destination.parent) as temporary:
            staging = Path(temporary) / 'payload'
            staging.mkdir()
            for member in members:
                target = staging / member.filename
                target.parent.mkdir(parents=True, exist_ok=True)
                with zipped.open(member) as incoming, target.open('xb') as outgoing:
                    shutil.copyfileobj(incoming, outgoing)
                target.chmod(0o755 if member.filename in BINARIES else 0o644)
            validate_directory(staging)
            staging.rename(destination)
    return destination


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--pack', type=Path, help='Pack a previously staged bundle')
    mode.add_argument('--unpack', type=Path, help='Validate and unpack our generated bundle')
    mode.add_argument('--validate', type=Path, help='Validate a staged directory without changing it')
    parser.add_argument('--macos-universal', type=Path)
    parser.add_argument('--linux-x64', type=Path)
    parser.add_argument('--windows-x64', type=Path)
    parser.add_argument('--output-dir', type=Path)
    parser.add_argument('--archive', type=Path)
    args = parser.parse_args()
    try:
        if args.validate:
            validate_directory(args.validate)
            print('Generated MCPB contract and inventory verified')
        elif args.pack:
            if not args.archive:
                parser.error('--pack requires --archive')
            print(pack_bundle(args.pack, args.archive))
        elif args.unpack:
            if not args.output_dir:
                parser.error('--unpack requires --output-dir')
            print(unpack_bundle(args.unpack, args.output_dir))
        else:
            if not all((args.macos_universal, args.linux_x64, args.windows_x64, args.output_dir)):
                parser.error('staging requires all three binaries and --output-dir')
            output = stage_bundle(args.macos_universal, args.linux_x64, args.windows_x64, args.output_dir)
            print(pack_bundle(output, args.archive) if args.archive else output)
    except (ValueError, OSError, zipfile.BadZipFile):
        parser.exit(1, 'MCPB operation failed: check the generated manifest, allowlisted inventory, paths and permissions.\n')


if __name__ == '__main__':
    main()
