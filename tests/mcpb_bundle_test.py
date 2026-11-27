"""The MCPB packer runs without npm and accepts only our generated bundle."""
import importlib.util
import json
from pathlib import Path
import stat
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('mcpb_package', ROOT / 'scripts/package-mcpb.py')
bundle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bundle)


class McpbBundleTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='memory-mcpb-')
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.source = self.base / 'source'
        self.source.mkdir()
        # A distinct version catches accidental hardcoding to the release that
        # happened to be current when this packer was introduced.
        self.write('Cargo.toml', '[package]\nname="project-memory-mcp"\nversion="8.7.6"\n')
        for name in ['README.md', 'LICENSE', 'SECURITY.md', 'docs/skills.md',
                     'docs/publishing.md', 'examples/mcp-config.json',
                     'scripts/install-skills.py', 'scripts/check-skills.py', 'scripts/backup-memory.py', 'scripts/verify-backup.py']:
            self.write(name, 'synthetic asset: ' + name)
        for name in ['project-memory', 'resume-project', 'record-decision', 'prepare-handoff']:
            self.write(f'skills/{name}/SKILL.md', '# Synthetic ' + name)
        self.write('skills/project-memory/references/security.md', '# Synthetic security workflow')
        self.write('skills/project-memory/agents/openai.yaml', 'interface:\n  display_name: Project Memory\n')
        self.write('docs/security.md', '# Synthetic security guide')
        self.root_patch = patch.object(bundle, 'ROOT', self.source)
        self.root_patch.start()
        self.addCleanup(self.root_patch.stop)
        self.inputs = []
        for name in ['mac', 'linux', 'windows']:
            path = self.base / name
            path.write_bytes((name + '-synthetic').encode())
            path.chmod(0o755)
            self.inputs.append(path)

    def write(self, name, content):
        path = self.source / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        return path

    def stage(self):
        destination = self.base / 'bundle'
        bundle.stage_bundle(*self.inputs, destination)
        return destination

    def assert_packer(self):
        self.assertTrue(callable(getattr(bundle, 'pack_bundle', None)), 'stdlib pack_bundle is not implemented')
        self.assertTrue(callable(getattr(bundle, 'unpack_bundle', None)), 'safe unpack_bundle is not implemented')

    def test_stages_native_platform_paths_and_all_skill_workflows(self):
        destination = self.stage()
        manifest = json.loads((destination / 'manifest.json').read_text())
        self.assertEqual(manifest['version'], '8.7.6')
        self.assertEqual(manifest['server']['type'], 'binary')
        config = manifest['server']['mcp_config']
        mapping = {'darwin': config['command'],
                   'linux': config['platform_overrides']['linux']['command'],
                   'win32': config['platform_overrides']['win32']['command']}
        for platform, payload in zip(['darwin', 'linux', 'win32'], ['mac', 'linux', 'windows']):
            executable = Path(mapping[platform].replace('${__dirname}', str(destination)))
            self.assertEqual(executable.read_bytes(), (payload + '-synthetic').encode())
        self.assertEqual({tool['name'] for tool in manifest['tools']}, {
            'create_project', 'list_projects', 'get_project_context', 'get_record',
            'save_note', 'search_notes', 'record_decision', 'save_handoff',
            'export_project', 'delete_project', 'search_memory', 'get_context_bundle', 'get_access_policy', 'save_review', 'get_review_history'})
        for name in ['project-memory', 'resume-project', 'record-decision', 'prepare-handoff']:
            self.assertTrue((destination / 'skills' / name / 'SKILL.md').is_file())
        self.assertTrue((destination / 'skills/project-memory/references/security.md').is_file())
        self.assertTrue((destination / 'scripts/install-skills.py').is_file())
        self.assertTrue((destination / 'scripts/backup-memory.py').is_file())
        self.assertTrue((destination / 'scripts/verify-backup.py').is_file())
        self.assertTrue((destination / 'skills/project-memory/agents/openai.yaml').is_file())
        self.assertTrue((destination / 'docs/security.md').is_file())

    def test_stages_new_cli_and_evaluation_guides_when_present(self):
        for guide in ('cli', 'skill-evaluation', 'retrieval-evaluation', 'access-policy', 'code-review', 'backup-verification'):
            self.write(f'docs/{guide}.md', f'# Synthetic {guide} guide')
        destination = self.stage()
        for guide in ('cli', 'skill-evaluation', 'retrieval-evaluation', 'access-policy', 'code-review', 'backup-verification'):
            self.assertEqual((destination / f'docs/{guide}.md').read_text(),
                             f'# Synthetic {guide} guide')

    def test_pack_unpack_roundtrip_preserves_native_bytes_and_executable_modes(self):
        self.assert_packer()
        destination = self.stage()
        archive = self.base / 'bundle.mcpb'
        bundle.pack_bundle(destination, archive)
        with zipfile.ZipFile(archive) as zipped:
            self.assertIn('manifest.json', zipped.namelist())
            executable = zipped.getinfo('server/darwin/project-memory-mcp')
            self.assertTrue((executable.external_attr >> 16) & stat.S_IXUSR)
        output = self.base / 'unpacked'
        bundle.unpack_bundle(archive, output)
        self.assertEqual((output / 'server/darwin/project-memory-mcp').read_bytes(), b'mac-synthetic')
        if sys.platform != 'win32':
            self.assertTrue((output / 'server/darwin/project-memory-mcp').stat().st_mode & stat.S_IXUSR)
        self.assertEqual((output / 'skills/project-memory/references/security.md').read_text(), '# Synthetic security workflow')

    def test_pack_rejects_unexpected_private_payload_without_creating_archive(self):
        self.assert_packer()
        destination = self.stage()
        (destination / '.env').write_text('synthetic private data')
        archive = self.base / 'bundle.mcpb'
        with self.assertRaises(ValueError):
            bundle.pack_bundle(destination, archive)
        self.assertFalse(archive.exists())

    def test_pack_rejects_changed_launch_command(self):
        self.assert_packer()
        destination = self.stage()
        path = destination / 'manifest.json'
        manifest = json.loads(path.read_text())
        manifest['server']['mcp_config']['command'] = '/bin/sh'
        path.write_text(json.dumps(manifest))
        with self.assertRaises(ValueError):
            bundle.pack_bundle(destination, self.base / 'bundle.mcpb')

    def test_pack_removes_partial_archive_when_reading_payload_fails(self):
        self.assert_packer()
        archive = self.base / 'failed.mcpb'
        directory = self.stage()
        with patch.object(bundle.shutil, 'copyfileobj', side_effect=OSError('synthetic read failure')):
            with self.assertRaises(OSError):
                bundle.pack_bundle(directory, archive)
        self.assertFalse(archive.exists())

    def test_pack_never_overwrites_an_existing_archive(self):
        self.assert_packer()
        archive = self.base / 'existing.mcpb'
        archive.write_bytes(b'previous release')
        with self.assertRaises(FileExistsError):
            bundle.pack_bundle(self.stage(), archive)
        self.assertEqual(archive.read_bytes(), b'previous release')

    @unittest.skipIf(sys.platform == 'win32', 'Symlink creation requires optional Windows privileges')
    def test_stage_rejects_symlink_input_and_skill_payload(self):
        linked = self.base / 'linked'
        linked.symlink_to(self.inputs[0])
        with self.assertRaises(ValueError):
            bundle.stage_bundle(linked, self.inputs[1], self.inputs[2], self.base / 'bad-input')
        self.assertFalse((self.base / 'bad-input').exists())
        self.write('skills/project-memory/external.md', 'placeholder').unlink()
        (self.source / 'skills/project-memory/external.md').symlink_to(self.inputs[0])
        with self.assertRaises(ValueError):
            self.stage()

    def test_unpack_rejects_traversal_duplicates_and_archive_links(self):
        self.assert_packer()
        for variant in ['traversal', 'duplicate', 'symlink']:
            with self.subTest(variant=variant):
                path = self.base / (variant + '.mcpb')
                with zipfile.ZipFile(path, 'w') as zipped:
                    if variant == 'traversal':
                        zipped.writestr('../escaped', 'payload')
                    elif variant == 'duplicate':
                        zipped.writestr('manifest.json', '{}')
                        import warnings
                        with warnings.catch_warnings():
                            warnings.simplefilter('ignore', UserWarning)
                            zipped.writestr('manifest.json', '{}')
                    else:
                        link = zipfile.ZipInfo('manifest.json')
                        link.create_system = 3
                        link.external_attr = (stat.S_IFLNK | 0o777) << 16
                        zipped.writestr(link, '/tmp/outside')
                destination = self.base / (variant + '-output')
                with self.assertRaises(ValueError):
                    bundle.unpack_bundle(path, destination)
                self.assertFalse(destination.exists())
                self.assertFalse((self.base / 'escaped').exists())


if __name__ == '__main__':
    unittest.main()
