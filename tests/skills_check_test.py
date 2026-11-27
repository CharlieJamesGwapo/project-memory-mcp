"""Validate skill frontmatter and bundled references without executing YAML."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
NAMES = ("project-memory", "resume-project", "record-decision", "prepare-handoff")


class SkillsCheckTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name).resolve()
        self.skills = self.base / "skills"
        for name in NAMES:
            folder = self.skills / name
            folder.mkdir(parents=True)
            (folder / "SKILL.md").write_text(f"---\nname: {name}\ndescription: Use when exercising this fixture.\n---\n# Test\n")
        self.router = self.skills / "project-memory/SKILL.md"
        reference = self.skills / "project-memory/references"
        reference.mkdir()
        (reference / "review.md").write_text("Evidence review.\n")
        with self.router.open("a") as target:
            target.write("[Review](references/review.md)\n[Resume](../resume-project/SKILL.md)\n")

    def check(self):
        return subprocess.run([sys.executable, str(ROOT / "scripts/check-skills.py"), "--skills-root", str(self.skills)], capture_output=True, text=True)

    def test_valid_pack_with_internal_and_sibling_references_passes(self):
        result = self.check()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_missing_reference_is_rejected(self):
        (self.skills / "project-memory/references/review.md").unlink()
        self.assertNotEqual(self.check().returncode, 0)

    def test_reference_outside_installed_pack_is_rejected(self):
        outside = self.base / "outside.md"
        outside.write_text("Do not bundle.\n")
        with self.router.open("a") as target:
            target.write("[Outside](../../outside.md)\n")
        self.assertNotEqual(self.check().returncode, 0)

    def test_required_name_must_match_folder(self):
        self.router.write_text("---\nname: different-name\ndescription: Use when testing.\n---\n# Test\n")
        self.assertNotEqual(self.check().returncode, 0)

    def test_duplicate_required_frontmatter_is_rejected(self):
        self.router.write_text("---\nname: project-memory\nname: project-memory\ndescription: Use when testing.\n---\n# Test\n")
        self.assertNotEqual(self.check().returncode, 0)

    def test_missing_description_is_rejected(self):
        self.router.write_text("---\nname: project-memory\n---\n# Test\n")
        self.assertNotEqual(self.check().returncode, 0)

    def test_yaml_object_tag_is_data_and_never_executed(self):
        sentinel = self.base / "executed"
        self.router.write_text("---\nname: project-memory\ndescription: !!python/object/apply:os.system [touch " + str(sentinel) + "]\n---\n# Test\n")
        result = self.check()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(sentinel.exists())

    @unittest.skipIf(sys.platform == "win32", "Symlinks require privileges on some Windows hosts")
    def test_symlinked_reference_is_rejected(self):
        reference = self.skills / "project-memory/references/review.md"
        reference.unlink()
        outside = self.base / "outside.md"
        outside.write_text("Do not read.\n")
        reference.symlink_to(outside)
        self.assertNotEqual(self.check().returncode, 0)


if __name__ == "__main__":
    unittest.main()
