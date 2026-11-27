import importlib.util
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
import zipfile
import os

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("release_package", ROOT / "scripts/package-release.py")
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseArchiveTest(unittest.TestCase):
    @unittest.skipIf(os.name == "nt", "Unix executable mode behavior")
    def test_unix_archive_preserves_executable_after_extract(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            binary = base / "project-memory-mcp"
            binary.write_text("#!/bin/sh\nexit 0\n")
            binary.chmod(0o755)
            archive = release.build_archive(binary, "aarch64-apple-darwin", base / "artifacts")
            extracted = base / "extracted"
            with tarfile.open(archive, "r:gz") as source:
                source.extractall(extracted, filter="data")
            executable = next(extracted.glob("*/project-memory-mcp"))
            self.assertTrue(os.access(executable, os.X_OK))
            self.assertEqual(subprocess.run([str(executable)]).returncode, 0)
            self.assertTrue((executable.parent / "skills/resume-project/SKILL.md").is_file())

    def test_windows_archive_keeps_exe_and_skills(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            binary = base / "project-memory-mcp.exe"
            binary.write_bytes(b"MZsynthetic")
            archive = release.build_archive(binary, "x86_64-pc-windows-msvc", base / "artifacts")
            self.assertEqual(archive.suffix, ".zip")
            with zipfile.ZipFile(archive) as source:
                exe = next(name for name in source.namelist() if name.endswith("/project-memory-mcp.exe"))
                self.assertEqual(source.read(exe), b"MZsynthetic")
                self.assertTrue(any(name.endswith("/skills/prepare-handoff/SKILL.md") for name in source.namelist()))
                names = source.namelist()
                for required in ("skills/project-memory/references/code-review.md", "scripts/install-skills.py", "scripts/backup-memory.py", "scripts/verify-backup.py", "docs/access-policy.md", "docs/code-review.md", "docs/backup-verification.md", "docs/setup.md", "docs/memory-health.md", "docs/mcp-views.md", "docs/architecture.md", "docs/api-reference.md", "CHANGELOG.md"):
                    self.assertTrue(any(name.endswith("/"+required) for name in names), required)


if __name__ == "__main__":
    unittest.main()
