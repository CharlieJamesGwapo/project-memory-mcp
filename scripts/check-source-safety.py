#!/usr/bin/env python3
"""Reviewable source policy checks, not antivirus or a complete secret detector.

Only Git-listed source files are read; ignored build output and user databases
are never recursively searched. Findings print rule IDs and locations, never
matching content. Run dependency advisory checks separately.
"""
import argparse
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tomllib

SECRET_PATTERNS = [
    re.compile(r'\bgh[pousr]_[A-Za-z0-9]{30,255}\b'),
    re.compile(r'\bgithub_pat_[A-Za-z0-9_]{60,255}\b'),
    re.compile(r'\bAKIA[0-9A-Z]{16}\b'),
    re.compile(r'-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----'),
]
RUNTIME_PATTERN = re.compile(
    r'\b(?:Command\s*::\s*new|std\s*::\s*net\s*::|reqwest\s*::|ureq\s*::|'
    r'TcpListener|TcpStream|UdpSocket|load_extension|unsafe\s*\{|'
    r'fetch\s*\(|XMLHttpRequest|WebSocket|navigator\.sendBeacon|'
    r'dangerouslySetInnerHTML|eval\s*\(|new\s+Function\s*\()'
)
REMOTE_PIPE = re.compile(r'\b(?:curl|wget)\b[^\n|]*\|\s*(?:sh|bash|zsh|pwsh|powershell)\b')
ACTION = re.compile(r'^[ \t]*-?[ \t]*[\'"]?uses[\'"]?:[ \t]*([^\n]*)$', re.MULTILINE)
PINNED_ACTION = re.compile(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_./-]+@[0-9a-f]{40}')
CRATES_SOURCE = 'registry+https://github.com/rust-lang/crates.io-index'
LOCAL_CRATES = {'project-memory-mcp', 'project-memory-studio'}
BINARY_SUFFIXES = {'.png', '.ico', '.icns', '.woff', '.woff2', '.ttf', '.otf'}


def check(root):
    result = subprocess.run(['git', '-C', str(root), 'ls-files', '--cached', '--others',
                             '--exclude-standard', '-z'], capture_output=True)
    if result.returncode:
        raise ValueError('The source root must be a Git working tree')
    names = sorted(set(p.decode('utf-8', 'surrogateescape') for p in result.stdout.split(b'\0') if p))
    findings = []

    def flag(name, rule, line=1):
        findings.append((name, line, rule))

    for name in names:
        relative = PurePosixPath(name)
        if relative.is_absolute() or '..' in relative.parts:
            flag(name, 'source-path')
            continue
        path = root / name
        # Check ancestors as well: an indexed directory could have been replaced
        # with a link in a dirty working tree. Do not follow it to private files.
        if any(part.is_symlink() for part in (path, *path.parents) if part != root and root in part.parents):
            flag(name, 'source-link')
            continue
        if not path.exists():
            continue  # A tracked deletion contains no current source to inspect.
        if not path.is_file():
            flag(name, 'source-file-type')
            continue
        if path.stat().st_size > 10 * 1024 * 1024:
            flag(name, 'source-size-review')
            continue
        if any(part in {'.env', 'credentials', 'node_modules', 'target'}
               or (part.startswith('.env.') and part != '.env.example')
               or part.endswith(('.sqlite', '.sqlite-wal', '.sqlite-shm', '.db', '.log'))
               for part in relative.parts):
            flag(name, 'private-file')
        if path.suffix.lower() in BINARY_SUFFIXES:
            continue
        try:
            text = path.read_text(encoding='utf-8')
        except (UnicodeError, OSError):
            flag(name, 'unreadable-source')
            continue
        for pattern in SECRET_PATTERNS:
            for match in pattern.finditer(text):
                flag(name, 'secret-pattern', text.count('\n', 0, match.start()) + 1)
        runtime = (name.startswith('src/') or name.startswith('apps/studio/src/')
                   or name.startswith('apps/studio/src-tauri/src/'))
        if runtime and '.test.' not in name:
            for match in RUNTIME_PATTERN.finditer(text):
                flag(name, 'runtime-boundary', text.count('\n', 0, match.start()) + 1)
        if name.startswith(('scripts/', '.github/')):
            for match in REMOTE_PIPE.finditer(text):
                flag(name, 'remote-execution', text.count('\n', 0, match.start()) + 1)
        if name.startswith('.github/workflows/') and path.suffix in {'.yml', '.yaml'}:
            for match in ACTION.finditer(text):
                reference = match.group(1).split('#', 1)[0].strip()
                if len(reference) >= 2 and reference[0] == reference[-1] and reference[0] in {'"', "'"}:
                    reference = reference[1:-1]
                if not PINNED_ACTION.fullmatch(reference):
                    flag(name, 'action-pin', text.count('\n', 0, match.start()) + 1)
            if re.search(r'\bnpm\s+exec\b[^\n]*--package(?:=|\s)', text):
                flag(name, 'unlocked-tool-execution')
        try:
            if path.name == 'package.json':
                package = json.loads(text)
                if {'preinstall', 'install', 'postinstall', 'prepare'} & set(package.get('scripts', {})):
                    flag(name, 'install-hook')
                if package.get('dependencies') or package.get('devDependencies'):
                    if path.with_name('package-lock.json').relative_to(root).as_posix() not in names:
                        flag(name, 'dependency-lock-missing')
            if path.name == 'package-lock.json':
                lock = json.loads(text)
                if lock.get('lockfileVersion') != 3 or not isinstance(lock.get('packages'), dict):
                    flag(name, 'dependency-lock-format')
                for location, package in lock.get('packages', {}).items():
                    if not location:
                        continue
                    if not package.get('resolved', '').startswith('https://registry.npmjs.org/'):
                        flag(name, 'dependency-source')
                    if not re.fullmatch(r'sha(?:256|384|512)-[A-Za-z0-9+/=]+', package.get('integrity', '')):
                        flag(name, 'dependency-integrity')
            if path.name == 'Cargo.lock':
                for package in tomllib.loads(text).get('package', []):
                    if 'source' not in package:
                        if package.get('name') not in LOCAL_CRATES:
                            flag(name, 'dependency-source')
                    elif package['source'] != CRATES_SOURCE:
                        flag(name, 'dependency-source')
                    elif not re.fullmatch(r'[0-9a-f]{64}', package.get('checksum', '')):
                        flag(name, 'dependency-integrity')
            if name == 'apps/studio/src-tauri/tauri.conf.json':
                csp = json.loads(text).get('app', {}).get('security', {}).get('csp')
                directives = {}
                if isinstance(csp, str):
                    for directive in csp.split(';'):
                        words = directive.split()
                        if words:
                            directives[words[0]] = set(words[1:])
                required = {'default-src': {"'self'"}, 'script-src': {"'self'"},
                            'connect-src': {'ipc:', 'http://ipc.localhost'},
                            'object-src': {"'none'"}, 'form-action': {"'none'"}}
                if any(directives.get(key) != values for key, values in required.items()):
                    flag(name, 'native-csp')
            if name.startswith('apps/studio/src-tauri/capabilities/') and path.suffix == '.json':
                capability = json.loads(text)
                if capability.get('permissions') != ['core:default'] or capability.get('windows') != ['main'] or capability.get('remote'):
                    flag(name, 'native-capability')
        except (ValueError, TypeError, AttributeError):
            flag(name, 'invalid-policy-input')
    return len(names), sorted(set(findings))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    try:
        count, findings = check(args.root.resolve())
    except (OSError, ValueError):
        print('Source policy could not run; use a readable Git working tree.', file=sys.stderr)
        return 2
    for name, line, rule in findings:
        print(f'{json.dumps(name, ensure_ascii=True)}:{line}: {rule}')
    if findings:
        print(f'Source policy failed: {len(findings)} finding(s); matching content is redacted.')
        return 1
    print(f'Source policy passed for {count} Git-listed files; this is not a malware or full secret scan.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
