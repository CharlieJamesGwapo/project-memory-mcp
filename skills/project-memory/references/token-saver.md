# Budgeted recovery

Read this route for a small project brief or explicit recovery budget. Recover
enough relevant evidence to proceed, then stop; do not export history by default.
Resolve the exact project UUID with `list_projects` when needed.

## v0.2 retrieval

Discover `get_context_bundle` and `search_memory` rather than assuming they are
available. Use a literal task term or known record IDs to avoid unrelated records.

`get_context_bundle` accepts `project_id`, optional `query` OR `record_ids`,
`max_bytes` (default 16000, range 4096–65536), and `max_records` (default 8,
range 1–20). Query and IDs cannot be combined. Explicit IDs must be 1–20 unique
canonical UUIDs in this project, fit the record cap, and fit the byte cap together.
The selection is atomic: invalid or oversized explicit selections return an
error, not a partial selection. Correct the input or discuss the necessary
scope; do not pretend an unsuccessful call retrieved the records.

Bundles return whole records with original authorship, sources, timestamps,
and replacement history plus `selection_reason`. A bundle record is already
complete; fetch it again only if a later change or missing record requires it.
Without query/IDs, the server considers latest handoff, active decisions, then
recent notes. Query mode uses literal current record text, not semantic ranking.
Automatic selection examines at most 200 candidates; it can skip a large record
to fit a later smaller one. Inspect `selection.candidate_window_truncated`,
`omitted_by_budget`, `omitted_by_record_limit`, and `selection_complete`.
Completeness covers that candidate pool, not all project history. A material
omission calls for a narrower search, explicit full record, or larger permitted
budget; name anything still unknown before relying on it.

`search_memory` accepts `project_id`, literal `query`, optional `kind`
(`note`, `decision`, `handoff`), `include_superseded` (default false), `limit`
(default 20, range 1–50), and `cursor`. It searches human-facing text across
record kinds. Results are previews; retrieve applicable complete records with
`get_record` or an explicit bundle. Reuse `next_cursor` only with the same project,
query, kind, and supersession filter. Search is ASCII case-insensitive; other
Unicode matches exactly. Include superseded history when explaining a change,
not as current instructions.

## Ranked terms and current policy

On v0.3+, discover the actual bundle schema before using optional
`query_mode:"terms"`. It requires a query, excludes explicit IDs and matches
one to eight deduplicated alphanumeric terms across human text. Ranking uses
distinct term coverage, headline coverage and recency; `matched_terms` explains
each match. This is lexical ranking, not confidence or semantic inference.
Literal remains the default; omit the new field for older servers that reject
unknown inputs. More matching records can mean more bytes. On v0.4, respect
the effective project/operation policy; a denied request does not authorize
retrieving the archive through a different path.

## Older-server fallback

Use `get_project_context` with a small `limit`, then `get_record` for relevant
handoff/decision previews. `limit` bounds history only: the response also has
active decision previews and latest handoff. `search_notes`, if available,
searches literal note text only. Follow history cursors when needed; never claim
unvisited history was read. No exact byte cap is available through this fallback.

## Report measured scope

The bundle cap covers the serialized MCP `CallToolResult`, including structured
JSON/text compatibility output and metadata; it excludes the outer JSON-RPC
envelope. `budget.used_bytes` reports those bytes. `budget.estimated_tokens` is
`ceil(used_bytes / 4)`, a heuristic, not a tokenizer result, model usage, or cost.
It does not cap this session's other tools, prompts, source reads, or answers.
Do not promise fixed savings. If measuring improvement, compare the same task,
records, and host using payload bytes/calls and separately available token data.

Return a brief with applicable decisions, current verified state, next action,
and material omissions. Read narrow source excerpts when current tools permit;
a historical test note remains historical. Reuse already read records and code.
