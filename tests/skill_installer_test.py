"""Exercise filesystem effects of the offline skill-pack installer."""
from pathlib import Path
import json
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
NAMES = ("project-memory", "resume-project", "record-decision", "prepare-handoff")


class SkillInstallerTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name).resolve()
        self.package = self.base / "package"
        self.source = self.package / "skills"
        self.script = self.package / "scripts/install-skills.py"
        self.script.parent.mkdir(parents=True)
        implementation = ROOT / "scripts/install-skills.py"
        if implementation.exists():
            shutil.copyfile(implementation, self.script)
        for name in NAMES:
            folder = self.source / name
            folder.mkdir(parents=True)
            (folder / "SKILL.md").write_text(
                f"---\nname: {name}\ndescription: Use when testing this pack.\n---\n# {name}\n",
                encoding="utf-8",
            )
        reference = self.source / "project-memory/references"
        reference.mkdir()
        (reference / "review.md").write_text("A bundled reference.\n", encoding="utf-8")
        self.destination = self.base / "user-skills"

    def run_install(self, *arguments, destination=None):
        return subprocess.run(
            [sys.executable, str(self.script), "--destination", str(destination or self.destination), *arguments],
            capture_output=True, text=True,
        )

    def installed(self):
        result = self.run_install("--apply")
        self.assertEqual(result.returncode, 0, result.stderr)

    def replacement_backup(self):
        self.installed()
        (self.destination / "resume-project/personal.md").write_text("Keep my notes.\n")
        result = self.run_install("--replace", "--apply")
        self.assertEqual(result.returncode, 0, result.stderr)
        backups = list(self.destination.glob(".project-memory-backup-*"))
        self.assertEqual(len(backups), 1)
        return backups[0]

    def test_default_dry_run_does_not_create_destination(self):
        result = self.run_install()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("dry-run", result.stdout.lower())
        self.assertFalse(self.destination.exists())

    def test_apply_installs_the_entire_pack_and_preserves_unrelated_files(self):
        self.destination.mkdir()
        (self.destination / "AGENTS.md").write_text("Existing project instructions.\n")
        (self.destination / "config.json").write_text('{"preserve":true}')
        self.installed()
        for name in NAMES:
            self.assertTrue((self.destination / name / "SKILL.md").is_file())
        self.assertEqual((self.destination / "project-memory/references/review.md").read_text(), "A bundled reference.\n")
        self.assertEqual((self.destination / "AGENTS.md").read_text(), "Existing project instructions.\n")
        self.assertEqual((self.destination / "config.json").read_text(), '{"preserve":true}')

    def test_identical_install_is_idempotent_without_backups(self):
        self.installed()
        result = self.run_install("--apply")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("unchanged", result.stdout.lower())
        self.assertEqual(list(self.destination.glob(".project-memory-backup-*")), [])

    def test_customized_folder_refuses_entire_apply_without_changes(self):
        target = self.destination / "resume-project"
        target.mkdir(parents=True)
        (target / "SKILL.md").write_text("Customized instructions.\n")
        result = self.run_install("--apply")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((target / "SKILL.md").read_text(), "Customized instructions.\n")
        self.assertFalse((self.destination / "project-memory").exists())

    def test_replace_dry_run_does_not_backup_or_remove_customization(self):
        self.installed()
        custom = self.destination / "resume-project/personal.md"
        custom.write_text("Keep me.\n")
        result = self.run_install("--replace")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(custom.read_text(), "Keep me.\n")
        self.assertEqual(list(self.destination.glob(".project-memory-backup-*")), [])

    def test_replace_backs_up_customized_folder_and_restore_recovers_it(self):
        backup = self.replacement_backup()
        self.assertEqual((backup / "resume-project/personal.md").read_text(), "Keep my notes.\n")
        self.assertFalse((self.destination / "resume-project/personal.md").exists())
        dry = self.run_install("--restore", str(backup))
        self.assertEqual(dry.returncode, 0, dry.stderr)
        self.assertFalse((self.destination / "resume-project/personal.md").exists())
        restored = self.run_install("--restore", str(backup), "--apply")
        self.assertEqual(restored.returncode, 0, restored.stderr)
        self.assertEqual((self.destination / "resume-project/personal.md").read_text(), "Keep my notes.\n")
        self.assertTrue((backup / "resume-project/personal.md").is_file())

    def test_restore_removes_new_pack_folders_that_did_not_preexist(self):
        prior = self.destination / "resume-project"
        prior.mkdir(parents=True)
        (prior / "SKILL.md").write_text("Old customized skill.\n")
        applied = self.run_install("--replace", "--apply")
        self.assertEqual(applied.returncode, 0, applied.stderr)
        backup = next(self.destination.glob(".project-memory-backup-*"))
        restored = self.run_install("--restore", str(backup), "--apply")
        self.assertEqual(restored.returncode, 0, restored.stderr)
        self.assertEqual((prior / "SKILL.md").read_text(), "Old customized skill.\n")
        for name in ("project-memory", "record-decision", "prepare-handoff"):
            self.assertFalse((self.destination / name).exists())

    def test_restore_refuses_to_discard_edits_made_after_install(self):
        backup = self.replacement_backup()
        current = self.destination / "resume-project/SKILL.md"
        current.write_text("New local edit after upgrade.\n")
        result = self.run_install("--restore", str(backup), "--apply")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(current.read_text(), "New local edit after upgrade.\n")

    def test_restore_rejects_manifest_paths_outside_the_pack(self):
        backup = self.replacement_backup()
        manifest = backup / "manifest.json"
        payload = json.loads(manifest.read_text())
        payload["actions"]["../outside"] = "install"
        manifest.write_text(json.dumps(payload))
        result = self.run_install("--restore", str(backup), "--apply")
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((self.destination / "project-memory/SKILL.md").is_file())

    def test_source_missing_one_skill_refuses_without_partial_install(self):
        shutil.rmtree(self.source / "record-decision")
        result = self.run_install("--apply")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.destination.exists())

    def test_existing_file_at_skill_name_is_never_replaced(self):
        self.destination.mkdir()
        collision = self.destination / "project-memory"
        collision.write_text("A user's file.\n")
        result = self.run_install("--replace", "--apply")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(collision.read_text(), "A user's file.\n")

    def test_missing_explicit_destination_is_a_usage_error(self):
        result = subprocess.run([sys.executable, str(self.script)], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)

    def test_destination_inside_source_is_rejected(self):
        result = self.run_install("--apply", destination=self.source / "nested")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.source / "nested").exists())

    @unittest.skipIf(sys.platform == "win32", "Symlinks require privileges on some Windows hosts")
    def test_symlinked_destination_ancestor_is_rejected(self):
        actual = self.base / "actual"
        actual.mkdir()
        alias = self.base / "alias"
        alias.symlink_to(actual, target_is_directory=True)
        result = self.run_install("--apply", destination=alias / "skills")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((actual / "skills").exists())

    @unittest.skipIf(sys.platform == "win32", "Symlinks require privileges on some Windows hosts")
    def test_symlinked_source_file_is_rejected_without_reading_target(self):
        private = self.base / "private"
        private.write_text("Do not distribute.\n")
        (self.source / "project-memory/leak.md").symlink_to(private)
        result = self.run_install("--apply")
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("Do not distribute.", result.stdout + result.stderr)
        self.assertFalse(self.destination.exists())

    @unittest.skipIf(sys.platform == "win32", "Symlinks require privileges on some Windows hosts")
    def test_symlinked_target_skill_is_rejected_even_with_replace(self):
        self.destination.mkdir()
        actual = self.base / "real-skill"
        actual.mkdir()
        (actual / "SKILL.md").write_text("Preserve.\n")
        (self.destination / "project-memory").symlink_to(actual, target_is_directory=True)
        result = self.run_install("--replace", "--apply")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((actual / "SKILL.md").read_text(), "Preserve.\n")


if __name__ == "__main__":
    unittest.main()
