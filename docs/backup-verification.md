# Verify a backup before restoring it

Use `verify-backup.py` to check that a Project Memory backup can be restored and
read. It opens the selected archive read-only, copies one consistent SQLite
snapshot into a private temporary directory, checks the copy, and removes it.
It never replaces your live archive.

Python 3.11 or newer is sufficient; the helper uses only the standard library.
From an extracted release or repository checkout:

```sh
python3 scripts/verify-backup.py \
  --source /absolute/path/project-memory-backup.sqlite
```

The default total deadline is 30 seconds. Choose a finite value greater than
zero and up to 3600 seconds with `--timeout`, for example `--timeout 120` for a
large backup. Lock waits, SQLite work, the copy and row comparisons share this
deadline. Filesystem calls and temporary-file cleanup still depend on the OS;
this is not a hard real-time process-kill deadline.

## What a successful check means

The helper checks:

- An absolute path to an existing regular file; final-component symlinks and
  directories are refused.
- The Project Memory application identity, schema version 1, expected tables
  and columns, SQLite integrity and foreign-key consistency.
- A real SQLite backup into a newly created temporary file, including committed
  data still held in the source WAL.
- Matching schema definitions and complete project, record and sequence rows
  between the source snapshot and the restored copy.
- Cleanup of the temporary restore before returning a successful report.

Success prints JSON with `status: "verified"`, project and record counts,
schema version, restored byte count and `content_match: true`. Record contents,
source filenames, credentials and SQL diagnostics are omitted from this report.
Failure exits nonzero without a success report. CLI errors use fixed descriptions
instead of echoing attacker-controlled SQLite schema messages.

Only the supported schema-1 table inventory is accepted. An archive with extra
application tables is refused rather than claiming those rows were verified.
Never remove unknown tables from a live archive simply to make this check pass.

## Restore deliberately

1. Create a backup using the [backup guide](data-safety.md).
2. Run this verifier on the backup you intend to restore.
3. Keep your current archive and existing backups intact. Copy the verified
   backup to a new absolute path, open that copy in Studio or inspect it with
   `project-memory-mcp --db /absolute/path/restored.sqlite doctor --json`.
4. Inspect the records you need before changing your normal client configuration.

The helper does not restore into a user-selected destination, overwrite an
archive, or change a client's configuration. Verification uses the snapshot
read during this run; later modifications require another check.

## Boundaries

This is a restore/readability check, not proof that a backup is authentic,
malware-free, current, or contains every record you expected. A valid archive
can contain false or malicious saved text. Counts and schema identity do not
establish who created it. Retain trusted release provenance and independently
validate important evidence.

Archives and temporary restored copies are plaintext. The temporary directory
is private, its archive is created with owner-only permissions where supported,
and it is deleted after the run; deletion is not secure erasure. SQLite may
maintain WAL/SHM coordination files even when opened read-only. Source main
database and WAL contents are not deliberately modified, and the helper does
not checkpoint the source. Another process with access to the archive or its
parent directory remains outside this helper's security boundary.
