# Troubleshooting

| Symptom | What to check |
|---|---|
| Terminal appears to wait | With no subcommand, the server waits for an MCP handshake. Use a client, `--help` or `doctor`. |
| Client cannot start the server | Use the extracted binary for your OS/CPU and an absolute executable path. Check executable permissions and OS trust controls. |
| Missing archive | Normal before first use. Start writable mode through your MCP client, or select an existing path. Read-only mode never creates a missing archive. |
| Relative path rejected | Use an absolute `--db` or `PROJECT_MEMORY_DB`; `--db` wins. GUI clients may have a different environment from your terminal. |
| Tools absent after installing files | MCP connection and skills are separate. Check the client's server discovery and documented refresh behavior. |
| New v0.2 tool absent | Check `--version` and the configured executable. A v0.1 server has the original ten tools. The skills retain a limited fallback. |
| Save/delete tool absent | Read-only mode intentionally exposes only seven read tools. Use a writable connection only when edits are wanted. |
| Duplicate project names | Resolve by UUID, purpose and directory. Names are not unique. |
| Search misses related concepts | Search is literal, not semantic. Try a phrase in the actual record. ASCII ignores case; other Unicode is exact. Source pointers and IDs are not searched. |
| Preview omits content | Fetch `get_record` with project and record IDs, or use a whole-record bundle. A preview is not the full record. |
| Bundle is empty or incomplete | Check budget/record omissions and the candidate-window flag. Empty selection does not prove the archive lacks relevant evidence. |
| `BUDGET_EXCEEDED` | Increase the permitted byte cap or choose fewer full records. Long project fields and compatibility text also count. No record is silently shortened. |
| `INVALID_CURSOR` | Reuse the cursor with the same operation, project and filters. Start a fresh query after changing them. |
| `FOREIGN_DATABASE` / `NEWER_SCHEMA` | Choose an actual supported Project Memory archive or upgrade. Do not change its identity/version fields manually. |
| `STORAGE_ERROR` / locked archive | Check permissions, competing writers and free disk space. SQLite waits briefly for locks. Retry after the competing operation finishes. |
| Skill installer collision | Preview first. Preserve customizations; use documented backed-up replacement only when intended. |
| Studio shows samples | Samples are synthetic. Open a real existing database in the desktop app. A browser preview has no live DB access. |
| Native save unavailable | Choose a writable existing folder and a `.md` filename. Expand the OS dialog if its compact view has not refreshed validation. Database/symlink targets are refused. |

## SQLite coordination files

Read-only mode prevents memory/schema writes and missing main-database creation.
SQLite can still create or update `-wal`/`-shm` coordination files beside a WAL
archive while opening it read-only. Doctor, Studio and the backup helper follow
SQLite's concurrency rules. They do not use `immutable=1` to bypass locking while
another MCP writer may be active. A folder with restricted permissions may need
existing readable sidecars for a WAL archive to open successfully.

## Collect a useful issue report

Include the binary version, OS/CPU, relevant `doctor --json` output with private
paths redacted, and a small synthetic reproduction. Include the exact error code
and which client/tool was involved. Remove secrets and private memory contents.
For sensitive security findings, follow [SECURITY.md](../SECURITY.md).

See [quickstart](quickstart.md), [API reference](api-reference.md),
[data safety](data-safety.md) and [skill installation](skills.md).
