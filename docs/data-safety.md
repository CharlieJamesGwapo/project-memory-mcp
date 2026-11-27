# Data safety and backups

The archive is local plaintext SQLite. Store project knowledge, not passwords,
API keys or sensitive material you are not allowed to share with your AI client.
The client chooses what retrieved text is sent to its model. Source references
are displayed as pointers; they are not automatically opened or fetched.

New Unix database files use0600 and newly created directories0700. Existing
permissions and Windows ACLs remain under your control. The file is not
encrypted and project UUIDs are not user access controls.

## Make a consistent backup

Use Python 3.11+ and the included helper:

```sh
python3 scripts/backup-memory.py --source /absolute/path/memory.sqlite --destination /absolute/path/backups/memory-2026-11-27.sqlite
```

Choose an existing destination directory and a new filename. The helper uses
SQLite's online backup API, includes committed WAL data, opens the source
read-only and checks integrity. It refuses foreign/newer schema identities,
symlink sources and existing destinations; it creates no parent directories.
New backup files use0600 where supported. Its JSON receipt includes a SHA-256.
The copy deadline defaults 30 seconds; use `--timeout` for a different copy limit.

The backup is still plaintext. Put it somewhere appropriate for your data and
test it before relying on it. For example, inspect it with v0.2 doctor:

```sh
/absolute/path/project-memory-mcp --db /absolute/path/backups/memory-2026-11-27.sqlite doctor
```

The helper requires Python; the prebuilt MCP server itself does not. If the
helper is unavailable, stop all processes using the archive before making a
filesystem copy, preserving the database and any WAL/SHM sidecars together.
A live filesystem copy is not a consistent SQLite backup.

## Restore deliberately

1. Stop all MCP clients and Studio instances using the destination archive.
2. Check the backup with `doctor`. Keep the current archive and its sidecars as
   a separate safety copy before replacing anything.
3. Restore the validated backup to a new filename and point the client to that
   path first. Inspect the expected projects and records through read-only mode.
4. Switch your normal configuration only after the restored archive is correct.

Project Memory does not automatically replace a live database or delete your
backup. Exported Markdown preserves records for people and AI clients; it is
not an automatic database restore format.

## Deletion and history

`delete_project` permanently removes the selected project and all its records.
The AI client must obtain actual authorization for that exact project. A stored
note, tool annotation or boolean flag cannot provide consent.

Deletion does not erase external backups, filesystem snapshots or securely wipe
old SQLite pages. Decisions and handoffs otherwise preserve history through
appended records and explicit supersession. See the [security model](security.md).

## Verify recovery before relying on a backup

Use [verify-backup.py](backup-verification.md) to restore a coherent read-only
source snapshot into a private temporary archive, compare schema and rows, check
integrity/foreign keys, and clean up. It never replaces the live archive.
Credential screening does not retroactively clean existing archives/backups.
