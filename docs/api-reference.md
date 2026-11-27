# MCP and CLI reference

v0.2 added two read tools while preserving the original ten contracts. v0.3 adds
optional ranked terms to context bundles and explicit CLI workflows. v0.4 adds local policy and review records. Default writable
mode exposes 15 tools; explicit deletion opt-in exposes 16; `--read-only` exposes ten.
Operation allowlists can reduce inventory further. Hosts may prefix tool names.
Unknown fields, invalid UUIDs and out-of-range values are rejected.

| Tool | Required arguments | Result / optional arguments |
|---|---|---|
| `create_project` | `name`, `purpose` | Project UUID; optional `directory` reference |
| `list_projects` | None | Project page; `limit`, `cursor` |
| `get_project_context` | `project_id` | Project, latest handoff, active decision previews and history page; `limit`, `cursor`, `include_superseded` |
| `get_record` | `project_id`, `record_id` | One complete record with provenance, sources and replacement metadata |
| `save_note` | `project_id`, `title`, `content`, `author_type` | New note; optional `sources` |
| `search_notes` | `project_id`, `query` | Literal title/content note previews; `limit`, `cursor` |
| `search_memory` | `project_id`, `query` | Literal note/decision/handoff previews; `kind`, `include_superseded`, `limit`, `cursor` |
| `get_context_bundle` | `project_id` | Whole-record selection; `query`/`query_mode` or `record_ids`, `max_bytes`, `max_records` |
| `record_decision` | `project_id`, `decision`, `rationale`, `author_type` | New decision; optional `sources`, `supersedes` |
| `save_handoff` | `project_id`, `summary`, `completed`, `unfinished`, `next_actions`, `blockers`, `verification`, `author_type` | New immutable handoff; optional `sources` |
| `export_project` | `project_id` | Full-record Markdown page, record IDs and next cursor; `limit`, `cursor` |
| `delete_project` | `project_id`, `confirm:true` | Disabled by default; local `allow_delete:true`, allowed operation and actual user authorization required |
| `get_access_policy` | None | Effective project/operation restrictions, read-only and screening mode, audit enabled status |
| `save_review` | `project_id`, `repository`, `base_commit`, `head_commit`, `summary`, `checks`, `findings`, `author_type` | Complete immutable review note; optional `sources`, `previous_review_id` |
| `get_review_history` | `project_id` | Complete review note page; `limit`, `cursor`, `current_head`, `max_bytes` |
| `get_memory_health` | `project_id` | Snapshot counts and complete assessment page; `limit`, `cursor`, `current_head`, `max_bytes` |

`author_type` is `user` or `assistant`. Sources are reference strings, not fetched
or independently verified evidence. Handoff work/verification fields are arrays
of strings. Only an explicit successor decision changes `superseded_by`.

## Search and pagination

Page limits default 20, range 1..50. Results have `items`, `next_cursor` and
`truncated`. Reuse the cursor with the same operation, project, exact query and
filters. Cursors preserve the original insertion snapshot; later inserts and
successors are excluded. Concurrent deletion can remove entries.

Search is literal: `%`, `_`, SQL punctuation, quotes, newlines and NUL do not
become query syntax. ASCII matching is case-insensitive; other Unicode matching
is exact. Unified search covers human-facing scalar and list text, including
rationale and handoff sections. It does not search IDs, authorship or source
pointers. `include_superseded` defaults false on unified search; `kind` can be
`note`, `decision` or `handoff`.

Previews have `full_record_required:true`. Fetch the relevant complete records.
Context headlines are bounded: up to 20 active decisions and a latest handoff
preview. Follow history/cursors when more decisions are present. Handoff
previews include up to five next actions at 300 characters each and a truncation
indicator. A complete-looking preview is still a preview.

## Whole-record context bundle

```json
{
  "project_id": "RETURNED-PROJECT-UUID",
  "query": "authentication",
  "max_bytes": 16000,
  "max_records": 8
}
```

The default `query_mode` is `literal`: a literal task phrase, not semantic search.
With no query, selection
prioritizes the latest handoff, active decisions newest first, then recent notes.
Automatic modes consider at most 200 candidates. Large records can be skipped
so smaller later records still fit. Records and project fields are never cut.

For words spread across fields, explicitly pass `query_mode:"terms"` with a
query. This mode extracts Unicode alphanumeric runs, folds ASCII case only,
deduplicates in input order and accepts 1–8 distinct terms. Punctuation-only
queries, more than eight distinct terms, or terms mode without a query fail
validation. Each term matches a literal substring in the same human-facing
fields as unified search; source pointers, IDs and authorship are excluded.
There is no stemming, synonym inference or embedding model.

Records matching any term are ranked across the whole project by distinct
matched-term count descending, then headline-term count in title/decision/summary,
then insertion sequence descending. Ranking happens before the 200-candidate
window. Superseded decisions remain excluded from automatic selection.
Terms results use `selection.mode:"terms"`, `selection.query_terms` and
`selection.ranking:"term_coverage_then_headline_then_recency"`; complete records
add `selection_reason:"ranked_term_match"` and ordered `matched_terms`.
Ranking explains lexical coverage, not semantic relevance or factual confidence.
Older v0.2 servers reject this new input field: inspect the discovered schema
before using it; omit `query_mode` for a compatible literal request.

Alternatively pass `record_ids` (1..20 unique IDs) to select complete records in
your chosen order, including historical decisions. This mode is atomic: all
IDs must belong to the project and fit both caps. It cannot be combined with
`query`. An empty explicit list, duplicate ID or partial fit is an error.

The result contains:

- `project`: complete project fields.
- `records`: complete record fields plus `selection_reason` (`explicit_selection`,
  `literal_query_match`, `ranked_term_match`, `latest_handoff`, `active_decision` or `recent_note`).
- `selection`: mode, candidate count, candidate-window truncation, omissions by
  budget/record cap, and `selection_complete`.
- `budget`: requested caps, `used_bytes`, `estimated_tokens` and `estimate_method`.
- `notice`: untrusted-data, historical verification and measurement boundaries.

`selection_complete` describes this candidate pool, not all project history.
`max_bytes` defaults 16000, range 4096..65536; `max_records` defaults 8, range 1..20.
An oversized project/header or explicit selection returns `BUDGET_EXCEEDED`.

**What the budget measures:** UTF-8 bytes of the canonical serialized SDK
`CallToolResult`, including structured data, normal serialized JSON text fallback
and accounting metadata. It excludes the outer JSON-RPC envelope. For an older
negotiated protocol the SDK strips `resultType`, reducing wire size by 24 bytes;
the canonical cap remains conservative. Token estimates are ceil(bytes/4), a
heuristic, not a model tokenizer, provider usage receipt or guaranteed savings.
The host may process the compatibility representations differently.

## Export

`export_project` returns full records in `structuredContent.markdown`, with
`record_ids`, `next_cursor` and `truncated`. The text block supplies a short
pointer to avoid duplicating large Markdown exports. Follow every cursor and
concatenate pages for the full export. Each page is a self-contained fenced
JSON document. Export is not a database backup or automatic restore format.

## Validation limits

| Input | Limit |
|---|---|
| Project name / note title | 200 Unicode scalar values |
| Project purpose / handoff summary | 4000 |
| Directory reference | 4096 |
| Note content | 16000 |
| Decision / rationale | 8000 each |
| Search query | 2000 |
| Each handoff list | 20 strings, each 1000 characters |
| Sources | 10 strings, each 2048 characters |
| Serialized record input | 65536 UTF-8 bytes, including JSON escaping |
| Serialized export MCP result | 256 KiB with 1 KiB reserved for protocol metadata |
| Studio Markdown brief | 20 records and 2 MiB UTF-8 |

## Errors and writes

Domain failures use `isError:true` and `structuredContent.error` with `code` and
`message`. Codes include `VALIDATION_ERROR`, `NOT_FOUND`, `INVALID_CURSOR`,
`CONFLICT`, `STORAGE_ERROR`, `FOREIGN_DATABASE`, `NEWER_SCHEMA`, `READ_ONLY`,
`BUDGET_EXCEEDED` and `INTEGRITY_ERROR`. SDK schema/unknown-tool failures can be
protocol errors. Internal SQL/OS details are not returned by storage errors.

Writes are atomic but have no idempotency keys. If delivery fails after a save,
inspect history before retrying. The server cannot establish human consent from
`confirm:true` or annotations; authorization belongs to the client and user.

## CLI

`--db ABSOLUTE_PATH` overrides `PROJECT_MEMORY_DB`; defaults are documented in
[quickstart](quickstart.md). With no subcommand, the process speaks MCP over
stdio. `--read-only` requires an existing archive and exposes only read tools.

`doctor [--json]` inspects identity, schema, SQLite integrity and project/record
counts without returning content or writing data. Exit codes: 0 ready, 2 missing,
1 error. `print-config` prints a JSON stdio example containing the current
executable and absolute DB path. It does not modify settings or create files.

v0.3 also provides explicit `init [--json]` and read-only `projects`, `search`,
`record` and `context [--json]`. See [the CLI guide](cli.md) for flags, UUIDs,
pagination, stdout/stderr and the distinction between canonical MCP budgets and
rendered CLI output. No-subcommand MCP behavior is unchanged.

## v0.4 policy and review contracts

See [local access policy](access-policy.md) for strict configuration, defaults, audit
semantics and credential boundaries. `ACCESS_DENIED`, `SECRET_DETECTED` and
`AUDIT_RECEIPT_FAILED` are new structured errors. Unsupported/malformed tool
arguments are sanitized; they never echo raw argument values.

See [code review](code-review.md) for complete field bounds, revision and freshness
contracts. Review history has its own limit 1..20 (default 10), canonical
byte range 4096..65536 (default 16000), complete records and scoped cursors.

CLI adds `--policy ABSOLUTE_JSON`, environment `PROJECT_MEMORY_POLICY`, `policy`,
and `inspect-secrets --project UUID [--limit N --cursor CURSOR]`. `print-config`
preserves the policy path. Inspection reports are sanitized; ordinary reads
continue to return complete original data.

## v0.5 setup, health and protocol views

`setup [--executable ABSOLUTE_PATH] [--json]` previews local checks and exact
client configuration. Default executable is this CLI. Exit 0 means its stated
checks passed; exit 2 means action is required, and invalid startup arguments
can exit 1. It does not execute a server, test a client connection or initialize
an archive. SQLite may maintain coordination files. [Setup contract](setup.md).

`health --project UUID [--current-head FULL_SHA --limit 20 --cursor CURSOR
--max-bytes 16000]` opens an existing archive read-only and calls
`get_memory_health`. Limits are 1..50 and 4096..65536 canonical bytes. Results
contain reference-only signals, not raw memory text or semantic conflict verdicts.
See [health](memory-health.md) for exact fields and snapshot coverage.

[MCP views](mcp-views.md) documents both resource templates and all three prompts,
strict arguments, policy enforcement, cache behavior and tool-based fallbacks.
