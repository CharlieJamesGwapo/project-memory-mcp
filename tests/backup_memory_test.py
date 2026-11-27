import importlib.util
from contextlib import closing
from pathlib import Path
import sqlite3
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "backup-memory.py"


class BackupMemoryTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "memory.sqlite"
        self.destination = self.root / "backup.sqlite"

    def load(self):
        self.assertTrue(SCRIPT.is_file(), "Online backup helper must exist")
        spec = importlib.util.spec_from_file_location("backup_memory", SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def archive(self):
        db = sqlite3.connect(self.source)
        db.executescript("PRAGMA journal_mode=WAL; PRAGMA application_id=1347240784; PRAGMA user_version=1; CREATE TABLE evidence(id INTEGER PRIMARY KEY, text TEXT); INSERT INTO evidence(text) VALUES('original');")
        return db

    def test_live_wal_backup_contains_committed_data_and_preserves_source(self):
        backup = self.load().backup_database
        writer = self.archive()
        self.addCleanup(writer.close)
        writer.execute("INSERT INTO evidence(text) VALUES(?)", ("日本語 latest committed",))
        writer.commit()
        before = self.source.read_bytes()
        report = backup(self.source, self.destination)
        self.assertEqual(report["status"], "ready")
        self.assertEqual(report["schema_version"], 1)
        self.assertEqual(self.source.read_bytes(), before)
        with closing(sqlite3.connect(self.destination)) as restored:
            self.assertEqual(restored.execute("SELECT text FROM evidence ORDER BY id").fetchall(), [("original",), ("日本語 latest committed",)])
            self.assertEqual(restored.execute("PRAGMA quick_check").fetchone()[0], "ok")

    def test_existing_destination_is_never_overwritten(self):
        backup = self.load().backup_database
        self.archive().close()
        self.destination.write_text("custom backup content")
        with self.assertRaises((ValueError, FileExistsError)):
            backup(self.source, self.destination)
        self.assertEqual(self.destination.read_text(), "custom backup content")

    def test_missing_and_foreign_sources_do_not_create_destinations(self):
        backup = self.load().backup_database
        with self.assertRaises((ValueError, FileNotFoundError)):
            backup(self.source, self.destination)
        self.assertFalse(self.destination.exists())
        self.source.write_bytes(b"unrelated file")
        with self.assertRaises((ValueError, sqlite3.DatabaseError)):
            backup(self.source, self.destination)
        self.assertFalse(self.destination.exists())
        self.assertEqual(self.source.read_bytes(), b"unrelated file")

    def test_foreign_sqlite_and_newer_schemas_are_not_treated_as_current_memory(self):
        backup = self.load().backup_database
        db = sqlite3.connect(self.source)
        db.execute("CREATE TABLE other(id INTEGER)")
        db.commit()
        db.close()
        before = self.source.read_bytes()
        with self.assertRaises(ValueError):
            backup(self.source, self.destination)
        self.assertEqual(self.source.read_bytes(), before)
        with closing(sqlite3.connect(self.source)) as db:
            db.executescript("PRAGMA application_id=1347240784; PRAGMA user_version=2;")
        with self.assertRaises(ValueError):
            backup(self.source, self.destination)
        self.assertFalse(self.destination.exists())

    @unittest.skipUnless(hasattr(Path, "symlink_to"), "symlinks unavailable")
    def test_symlinks_and_relative_paths_are_refused(self):
        backup = self.load().backup_database
        self.archive().close()
        alias = self.root / "alias.sqlite"
        try:
            alias.symlink_to(self.source)
        except OSError:
            self.skipTest("account cannot create symlinks")
        with self.assertRaises(ValueError):
            backup(alias, self.destination)
        with self.assertRaises(ValueError):
            backup(Path("relative.sqlite"), self.destination)
        self.assertFalse(self.destination.exists())


if __name__ == "__main__":
    unittest.main()
