# Local access policy

v0.4 adds server-enforced capability restrictions. Default writable startup exposes
14 tools: deletion is disabled. A local policy can enable deletion, exposing 15.
Read-only mode exposes nine tools. An operation allowlist can reduce these counts.
The MCPB manifest describes the full optional inventory; live discovery is authoritative.

## Start safely

Inspect defaults without opening an archive:

```sh
project-memory-mcp policy
```

Save the following as an absolute local JSON file, replacing the example project
UUID with the exact ID returned by `projects`. JSON does not support comments.

```json
{
  "projects": ["00000000-0000-4000-8000-000000000001"],
  "operations": ["list_projects", "get_project_context", "get_record", "search_notes", "search_memory", "get_context_bundle", "get_review_history", "get_access_policy"],
  "allow_delete": false,
  "secrets": "reject"
}
```

```sh
project-memory-mcp --db /absolute/path/memory.sqlite --policy /absolute/path/policy.json print-config
project-memory-mcp --db /absolute/path/memory.sqlite --policy /absolute/path/policy.json --read-only
```

`print-config` preserves the policy argument and read-only flag, prints JSON and
does not change client settings. `PROJECT_MEMORY_POLICY` is the environment
alternative. Restart the connection to load policy changes. Unknown keys, unknown
operations, malformed JSON, noncanonical UUIDs, files over 65536 bytes, relative
paths and final-component policy symlinks are rejected before archive opening.

## Fields and enforcement

| Field | Default | Meaning |
|---|---|---|
| `projects` | Omitted/null | All projects; an array restricts UUIDs, `[]` allows none. Restricted scope disables project creation. |
| `operations` | Omitted/null | All supported operations subject to other restrictions; `[]` allows none. |
| `allow_delete` | `false` | Explicit local opt-in; still requires allowed operation, writable mode and actual user authorization. |
| `secrets` | `"reject"` | Reject matched credentials on writes and `export_project`; `"allow"` opts out for deliberate local use. |
| `audit_path` | Omitted/null | Absolute metadata audit JSONL path; parent directory must exist. |

Store enforces restrictions before dispatch. Discovery removes unavailable tools;
calling a hidden tool does not bypass the Store guard. Project lists and diagnostic
counts include permitted projects. Internal operations `studio_list_projects`,
`studio_list_records` and `inspect_secrets` require their own entries if using an
explicit operations allowlist. They are not extra public MCP tools.

This is per-connection local configuration, not authentication or multi-user
tenancy. A person/process with archive access can start another unrestricted
connection, open Studio directly, change local policy or read SQLite. Pagination
cursors contain global insertion sequence values, which can reveal archive
activity counts despite filtering record contents. Use separate archives and OS
permissions when project activity or data must be confidential from another client.
Studio opens its own local read-only connection; it does not inherit another
process's `--policy` file.

## Credential screening and export inspection

The detector recognizes common private-key PEM headers, GitHub personal-token
patterns, AWS `AKIA` access-key identifiers and sufficiently long `sk-` API-key
patterns. It checks nested fields and source strings. Patterns are intentionally
narrow and can have false positives and false negatives. Errors/report paths omit
credential values; invalid tool argument errors are sanitized too.

Inspect existing/imported records without printing their contents:

```sh
project-memory-mcp --db /absolute/path/memory.sqlite inspect-secrets --project RETURNED-UUID --limit 20
```

Follow `next_cursor` with `--cursor` until null. Reports include project metadata
findings and each inspected record UUID with categories/structural paths, capped at
100 findings per inspected value. They are not a comprehensive DLP or malware scan.
The operation allowlist must permit `inspect_secrets`.

Default rejection applies to new writes, MCP `export_project` and CLI `context`
output. Existing records are never silently redacted or rewritten. Ordinary MCP
reads/context retrieval, CLI `record`/`search` and Studio can still return complete
previously stored credentials to an authorized reader. The AI host controls
transmission to its model/provider. Inspect and remove sensitive data according
to your retention policy; backups and filesystem snapshots remain separate.

## Optional audit receipts

Add `"audit_path":"/absolute/private/path/audit.jsonl"` to local policy. New files
use mode 0600 on Unix. Existing files must have the expected format header;
symlinks, hardlinks on Unix, unrelated files and archive/SQLite-sidecar aliases
are refused. Conservative case/Unicode aliases are refused even on filesystems
where those names would be distinct. Audit preparation occurs before SQLite initialization.

Receipts contain time, operation, syntactically valid project UUID and outcome.
They exclude record contents, titles, query strings, source references and secret
values. A durable attempt is written before an operation; audit failure at that
point prevents mutation. Successful and failed operation outcomes follow.

The audit and database are separate files, not an atomic transaction. An attempt
without an outcome is inconclusive after a crash. `AUDIT_RECEIPT_FAILED` means the
operation may already have committed: inspect history before retrying a write.
Pre-routing argument failures do not generate Store audit events. Concurrent
processes do not provide a single total audit ordering. The file is not tamper-proof
against its local owner; retain it securely and rotate only while clients are stopped.

See [security](security.md), [review records](code-review.md),
[backup verification](backup-verification.md) and [CLI](cli.md).
