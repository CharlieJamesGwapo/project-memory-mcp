#!/usr/bin/env python3
"""Verify a Project Memory backup by restoring and checking a disposable copy."""
import argparse
from contextlib import closing
import json
import math
import os
from pathlib import Path
import sqlite3
import stat
import tempfile
import time


PROJECT_COLUMNS = "seq,id,name,purpose,directory,created_at"
RECORD_COLUMNS = "seq,id,project_id,kind,data,created_at,supersedes"


def verify_backup(source, timeout=30):
    """Read one snapshot, restore it privately, compare it and remove the restore.

    The source is opened read-only. SQLite may maintain WAL/SHM coordination
    files; no source records, schema or main/WAL bytes are deliberately changed.
    A separate process with direct file access remains outside this boundary.
    """
    started = time.monotonic()
    if not isinstance(timeout, (int, float)) or not math.isfinite(timeout) or not 0 < timeout <= 3600:
        raise ValueError("Timeout must be finite and greater than 0, up to 3600 seconds")
    deadline = started + timeout

    def remaining():
        budget = deadline - time.monotonic()
        if budget <= 0:
            raise TimeoutError("Backup verification deadline exceeded")
        return budget

    def progress():
        return int(time.monotonic() >= deadline)

    def execute(conn, sql):
        # Refresh before every statement: separate lock waits share one deadline.
        conn.execute(f"PRAGMA busy_timeout={max(0, int(remaining() * 1000))}")
        return conn.execute(sql)

    def identity(conn):
        application = execute(conn, "PRAGMA application_id").fetchone()[0]
        version = execute(conn, "PRAGMA user_version").fetchone()[0]
        if application != 1347240784 or version != 1:
            raise ValueError("Source is not a supported Project Memory schema 1 archive")
        tables = {row[0] for row in execute(conn, "SELECT name FROM sqlite_schema WHERE type='table'")}
        if not {"projects", "records", "sqlite_sequence"}.issubset(tables):
            raise ValueError("Archive is missing the expected memory tables")
        if tables != {"projects", "records", "sqlite_sequence"}:
            raise ValueError("Archive contains tables outside the supported memory schema")
        execute(conn, f"SELECT {PROJECT_COLUMNS} FROM projects LIMIT 0")
        execute(conn, f"SELECT {RECORD_COLUMNS} FROM records LIMIT 0")
        return version

    def integrity(conn):
        if execute(conn, "PRAGMA integrity_check(1)").fetchone()[0] != "ok":
            raise ValueError("Archive SQLite integrity check failed")
        if execute(conn, "PRAGMA foreign_key_check").fetchone() is not None:
            raise ValueError("Archive contains inconsistent foreign-key references")

    def compare_rows(src, dst, sql):
        left, right = execute(src, sql), execute(dst, sql)
        while True:
            remaining()
            a, b = left.fetchmany(128), right.fetchmany(128)
            if a != b:
                raise ValueError("Temporary restore contents do not match the source snapshot")
            if not a:
                return

    source = Path(source)
    if not source.is_absolute():
        raise ValueError("Source must be an absolute path")
    metadata = source.lstat()
    if not stat.S_ISREG(metadata.st_mode):
        raise ValueError("Source must be a regular file, not a symlink or directory")

    # URI mode=ro never creates a missing source or obtains a read-write handle.
    try:
        with closing(sqlite3.connect(source.as_uri() + "?mode=ro", uri=True,
                                    timeout=remaining())) as src:
            src.set_progress_handler(progress, 1000)
            execute(src, "PRAGMA query_only=ON")
            execute(src, "BEGIN")
            version = identity(src)
            integrity(src)
            with tempfile.TemporaryDirectory(prefix="project-memory-verify-") as directory:
                restored = Path(directory) / "restored.sqlite"
                descriptor = os.open(restored, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
                os.close(descriptor)
                with closing(sqlite3.connect(restored.as_uri() + "?mode=rw", uri=True,
                                            timeout=remaining())) as dst:
                    dst.set_progress_handler(progress, 1000)

                    def backup_progress(_status, _remaining, _total):
                        remaining()

                    src.backup(dst, pages=128, progress=backup_progress,
                               sleep=min(0.01, remaining()))
                    identity(dst)
                    integrity(dst)
                    compare_rows(src, dst,
                                 "SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY type,name")
                    compare_rows(src, dst, f"SELECT {PROJECT_COLUMNS} FROM projects ORDER BY seq")
                    compare_rows(src, dst, f"SELECT {RECORD_COLUMNS} FROM records ORDER BY seq")
                    # AUTOINCREMENT high-water values affect future inserts as well.
                    compare_rows(src, dst, "SELECT name,seq FROM sqlite_sequence ORDER BY name")
                    project_count = execute(dst, "SELECT COUNT(*) FROM projects").fetchone()[0]
                    record_count = execute(dst, "SELECT COUNT(*) FROM records").fetchone()[0]
                restored_bytes = restored.stat().st_size
            remaining()
        return {"status": "verified", "schema_version": version,
                "project_count": project_count, "record_count": record_count,
                "restored_bytes": restored_bytes, "content_match": True,
                "temporary_restore_removed": True,
                "notice": "Verified a disposable SQLite restore; this does not authenticate the backup or replace your live archive."}
    except sqlite3.Error as error:
        if time.monotonic() >= deadline:
            raise TimeoutError("Backup verification deadline exceeded") from error
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True,
                        help="Absolute path to a backup archive; opened read-only")
    parser.add_argument("--timeout", type=float, default=30,
                        help="Total verification deadline in seconds, greater than 0 up to 3600 (default: 30)")
    args = parser.parse_args()
    try:
        report = verify_backup(args.source, args.timeout)
    except (ValueError, TimeoutError) as error:
        parser.exit(1, f"Verification failed: {error}\n")
    except sqlite3.Error:
        # SQLite messages can describe attacker-controlled schema. Keep CLI output
        # independent of database strings, including errors and constraint text.
        parser.exit(1, "Verification failed: SQLite validation or restore could not complete\n")
    except OSError:
        parser.exit(1, "Verification failed: cannot access the source or private temporary restore\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
