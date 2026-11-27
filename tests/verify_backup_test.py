import importlib.util
import json
import math
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "verify-backup.py"
SCHEMA = """
PRAGMA application_id=1347240784;
PRAGMA user_version=1;
PRAGMA foreign_keys=ON;
CREATE TABLE projects (
 seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT NOT NULL UNIQUE,
 name TEXT NOT NULL, purpose TEXT NOT NULL, directory TEXT, created_at TEXT NOT NULL
);
CREATE TABLE records (
 seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT NOT NULL UNIQUE,
 project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 kind TEXT NOT NULL CHECK(kind IN ('note','decision','handoff')),
 data TEXT NOT NULL CHECK(json_valid(data)), created_at TEXT NOT NULL,
 supersedes TEXT UNIQUE, UNIQUE(project_id,id),
 FOREIGN KEY(project_id,supersedes) REFERENCES records(project_id,id)
);
CREATE INDEX records_project_seq ON records(project_id,seq DESC);
"""


class VerifyBackupTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "backup with spaces.sqlite"

    def load(self):
        self.assertTrue(SCRIPT.is_file(), "Backup recovery verification helper must exist")
        spec = importlib.util.spec_from_file_location("verify_backup", SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def archive(self, wal=False):
        conn = sqlite3.connect(self.source)
        if wal:
            conn.execute("PRAGMA journal_mode=WAL")
            conn.execute("PRAGMA wal_autocheckpoint=0")
        conn.executescript(SCHEMA)
        conn.execute("INSERT INTO projects(id,name,purpose,created_at) VALUES(?,?,?,?)",
                     ("p", "Synthetic", "Private source text must not be reported", "2026-11-27"))
        conn.execute("INSERT INTO records(id,project_id,kind,data,created_at) VALUES(?,?,?,?,?)",
                     ("r", "p", "note", json.dumps({"content": "SENSITIVE-TEST-CONTENT 日本語"}), "2026-11-27"))
        conn.commit()
        return conn

    def test_restore_verifies_schema_data_and_foreign_keys_without_retaining_copy(self):
        self.archive().close()
        before = self.source.read_bytes()
        before_files = set(self.root.iterdir())
        report = self.load().verify_backup(self.source)
        self.assertEqual(report["status"], "verified")
        self.assertEqual(report["project_count"], 1)
        self.assertEqual(report["record_count"], 1)
        self.assertEqual(report["schema_version"], 1)
        self.assertTrue(report["content_match"])
        self.assertTrue(report["temporary_restore_removed"])
        self.assertNotIn("SENSITIVE-TEST-CONTENT", json.dumps(report))
        self.assertNotIn("Private source text", json.dumps(report))
        self.assertEqual(self.source.read_bytes(), before)
        self.assertEqual(set(self.root.iterdir()), before_files)

    def test_live_wal_committed_rows_are_verified_without_checkpointing_source(self):
        writer = self.archive(wal=True)
        self.addCleanup(writer.close)
        writer.execute("INSERT INTO records(id,project_id,kind,data,created_at) VALUES(?,?,?,?,?)",
                       ("latest", "p", "handoff", '{"summary":"latest committed"}', "2026-11-27"))
        writer.commit()
        wal = Path(str(self.source) + "-wal")
        before = self.source.read_bytes(), wal.read_bytes()
        report = self.load().verify_backup(self.source)
        self.assertEqual(report["record_count"], 2)
        self.assertEqual((self.source.read_bytes(), wal.read_bytes()), before)

    def test_missing_relative_symlink_and_non_file_sources_are_rejected(self):
        verify = self.load().verify_backup
        with self.assertRaises((ValueError, OSError)):
            verify(self.source)
        self.assertFalse(self.source.exists())
        with self.assertRaises(ValueError):
            verify(Path("relative.sqlite"))
        with self.assertRaises(ValueError):
            verify(self.root)
        self.archive().close()
        alias = self.root / "alias.sqlite"
        try:
            alias.symlink_to(self.source)
        except OSError:
            return  # Windows accounts may lack symlink privileges; other checks ran.
        with self.assertRaises(ValueError):
            verify(alias)

    def test_foreign_newer_and_forged_identity_without_memory_tables_are_rejected(self):
        verify = self.load().verify_backup
        for sql in [
            "CREATE TABLE foreign_table(value TEXT)",
            "PRAGMA application_id=1347240784; PRAGMA user_version=2;",
            "PRAGMA application_id=1347240784; PRAGMA user_version=1; CREATE TABLE wrong(value TEXT);",
        ]:
            with self.subTest(sql=sql):
                if self.source.exists():
                    self.source.unlink()
                conn = sqlite3.connect(self.source)
                conn.executescript(sql)
                conn.close()
                before = self.source.read_bytes()
                with self.assertRaises((ValueError, sqlite3.Error)):
                    verify(self.source)
                self.assertEqual(self.source.read_bytes(), before)

    def test_corruption_and_foreign_key_inconsistency_do_not_report_success(self):
        verify = self.load().verify_backup
        self.source.write_bytes(b"not SQLite: SENSITIVE-TEST-CONTENT")
        before = self.source.read_bytes()
        with self.assertRaises((ValueError, sqlite3.Error)):
            verify(self.source)
        self.assertEqual(self.source.read_bytes(), before)
        self.source.unlink()
        conn = self.archive()
        conn.execute("PRAGMA foreign_keys=OFF")
        conn.execute("UPDATE records SET project_id='missing'")
        conn.commit()
        conn.close()
        with self.assertRaises(ValueError):
            verify(self.source)

    def test_unexpected_user_table_is_not_silently_omitted_from_content_verification(self):
        conn = self.archive()
        conn.execute("CREATE TABLE unchecked(text TEXT)")
        conn.execute("INSERT INTO unchecked VALUES('SENSITIVE-TEST-CONTENT')")
        conn.commit()
        conn.close()
        before = self.source.read_bytes()
        with self.assertRaises(ValueError):
            self.load().verify_backup(self.source)
        self.assertEqual(self.source.read_bytes(), before)

    def test_changed_restored_content_is_rejected_and_temporary_copy_is_removed(self):
        self.archive().close()
        module = self.load()
        real_connect = sqlite3.connect
        real_temporary_directory = tempfile.TemporaryDirectory
        restore_directories = []

        class PoisonedSource(sqlite3.Connection):
            def backup(self, destination, **kwargs):
                super().backup(destination, **kwargs)
                destination.execute("UPDATE records SET data='{}'")
                destination.commit()

        def connect(database, **kwargs):
            if "mode=ro" in database:
                kwargs["factory"] = PoisonedSource
            return real_connect(database, **kwargs)

        def temporary_directory(**kwargs):
            result = real_temporary_directory(dir=self.root, **kwargs)
            restore_directories.append(Path(result.name))
            return result

        with patch.object(module.sqlite3, "connect", side_effect=connect), \
                patch.object(module.tempfile, "TemporaryDirectory", side_effect=temporary_directory):
            with self.assertRaisesRegex(ValueError, "contents do not match"):
                module.verify_backup(self.source)
        self.assertEqual(len(restore_directories), 1)
        self.assertFalse(restore_directories[0].exists())

    def test_invalid_timeouts_are_rejected_and_elapsed_timeout_does_not_succeed(self):
        verify = self.load().verify_backup
        self.archive().close()
        for timeout in [0, -1, math.inf, math.nan, 3601]:
            with self.subTest(timeout=timeout), self.assertRaises(ValueError):
                verify(self.source, timeout=timeout)
        with self.assertRaises(TimeoutError):
            verify(self.source, timeout=1e-12)

    def test_busy_database_deadline_is_bounded(self):
        verify = self.load().verify_backup
        writer = self.archive()
        self.addCleanup(writer.close)
        writer.execute("BEGIN EXCLUSIVE")
        started = time.monotonic()
        with self.assertRaises((sqlite3.Error, TimeoutError)):
            verify(self.source, timeout=0.08)
        self.assertLess(time.monotonic() - started, 1.0)

    def test_cli_success_is_json_and_failure_does_not_echo_database_values(self):
        self.archive().close()
        completed = subprocess.run([sys.executable, str(SCRIPT), "--source", str(self.source)],
                                   text=True, capture_output=True, check=False)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(json.loads(completed.stdout)["status"], "verified")
        self.source.write_bytes(b"not SQLite: SENSITIVE-TEST-CONTENT")
        failed = subprocess.run([sys.executable, str(SCRIPT), "--source", str(self.source)],
                                text=True, capture_output=True, check=False)
        self.assertNotEqual(failed.returncode, 0)
        self.assertFalse(failed.stdout)
        self.assertIn("Verification failed", failed.stderr)
        self.assertNotIn("SENSITIVE-TEST-CONTENT", failed.stderr)


if __name__ == "__main__":
    unittest.main()
