#!/usr/bin/env python3
"""Create a consistent SQLite backup without changing the source or replacing files."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import stat
import time


def backup_database(source, destination, timeout=30):
    source, destination = Path(source), Path(destination)
    if not source.is_absolute() or not destination.is_absolute():
        raise ValueError("Source and destination must be absolute paths")
    if not stat.S_ISREG(source.lstat().st_mode):
        raise ValueError("Source must be a regular file, not a symlink or directory")
    if timeout <= 0:
        raise ValueError("Backup timeout must be positive")
    src = sqlite3.connect(source.as_uri() + "?mode=ro", uri=True, timeout=5)
    owned = None
    dst = None
    try:
        identity = src.execute("PRAGMA application_id").fetchone()[0]
        version = src.execute("PRAGMA user_version").fetchone()[0]
        if identity != 1347240784 or version != 1:
            raise ValueError("Source is not a supported Project Memory schema1 archive")
        if src.execute("PRAGMA quick_check(1)").fetchone()[0] != "ok":
            raise ValueError("Source SQLite integrity check failed")
        # Exclusive creation refuses existing files, hardlinks and dangling symlinks.
        fd = os.open(destination, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
        owned = os.fstat(fd)
        os.close(fd)
        dst = sqlite3.connect(destination.as_uri() + "?mode=rw", uri=True, timeout=5)
        deadline = time.monotonic() + timeout

        def progress(_status, _remaining, _total):
            if time.monotonic() > deadline:
                raise TimeoutError("Backup exceeded its time limit; retry when the archive is less busy")

        src.backup(dst, pages=128, progress=progress, sleep=0.05)
        if dst.execute("PRAGMA quick_check(1)").fetchone()[0] != "ok":
            raise ValueError("Backup integrity check failed")
        dst.close()
        dst = None
        digest = hashlib.sha256()
        with destination.open("rb") as stream:
            for chunk in iter(lambda: stream.read(65536), b""):
                digest.update(chunk)
        return {"status": "ready", "source": str(source), "destination": str(destination),
                "schema_version": version, "bytes": destination.stat().st_size,
                "sha256": digest.hexdigest()}
    except BaseException:
        if dst is not None:
            dst.close()
        if owned is not None:
            try:
                current = destination.lstat()
                if (current.st_dev, current.st_ino) == (owned.st_dev, owned.st_ino):
                    destination.unlink()
            except FileNotFoundError:
                pass
        raise
    finally:
        src.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--timeout", type=float, default=30, help="Backup copy deadline in seconds (default30)")
    args = parser.parse_args()
    try:
        report = backup_database(args.source, args.destination, args.timeout)
    except (ValueError, OSError, sqlite3.Error, TimeoutError) as error:
        parser.exit(1, f"Backup failed: {error}\n")
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
