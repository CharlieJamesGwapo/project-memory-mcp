# v0.5.0 guided setup and memory health

The user requested continued improvements after the v0.5 recommendation and has
already authorized testing, GitHub pushes and free Registry publication. Keep
Rust 1.100, SQLite schema 1, complete records, immutable history and local stdio.

## Product decisions

Deliver one coherent upgrade: setup preview, Studio review/health views and
read-only MCP resources plus reusable prompts. Prefer these existing archive
and review contracts over a new semantic database, autonomous code runner or
repository crawler. Source freshness remains supplied-commit comparison; no
automatic file crawl or model inference is introduced.

### Setup preview

Shared `setup::preview(database, executable, policy, policy_path, read_only)`
returns structured diagnostics, exact JSON stdio configuration and next actions.
Paths are explicit absolute paths. Inspect executable metadata only, never run
it or claim a connection was tested. Missing archive produces an actionable
report without creating it. Invalid/foreign archives remain untouched. Policy
is inspected without creating/appending audit logs. As with doctor/Studio, SQLite
may create/update WAL/SHM coordination files; read-only describes memory/schema
and does not promise an unchanged filesystem tree. A configured policy path
and the read-only flag must survive generated configuration. Native UI reads
explicitly entered paths, displays/copies the JSON and never initializes an
archive or changes another client's settings. Samples are labeled synthetic.

### Health report

Add strict `input::MemoryHealth` with project_id, current_head(optional40/64hex),
limit(default20,1..50), cursor and max_bytes(default16000,4096..65536).
Store method `memory_health` returns project_id, summary, complete per-record
assessments `items`, next_cursor/truncated, records_examined, budget and notice.
Each item contains record_id, kind and signals (code/message only, no memory text
or credential values). Summary counts are from the same insertion snapshot.
Scope cursors to project/current_head. Keep complete assessment items within
canonical serialized tool-result budget; page through all records even when
signals are empty. No automatic repair/write/delete.

Signals may identify missing source references, repeated literal note text,
current review follow-up statuses, a supplied review-head mismatch and an
unfinished latest handoff without next actions. Multiple active decisions are
a review suggestion, never proof of a semantic conflict. A clean/bounded page
cannot establish a healthy entire archive. Explicitly disclose historical and
incomplete evidence; do not print matched credentials.

### Studio

Retain the established visual system. Add Reviews and Health tabs with bounded
pagination, exact head input and full review display of commits/findings/checks/
revision/freshness. Preserve manual context selection until explicit Add.
Guard stale project/session/request responses and invalidate mismatched query
results. Use existing full-note representations and explicit inspector/Add for
selected reviews. Native whitelist adds get_review_history, get_memory_health
and get_access_policy. Studio is its own local read-only connection; setup
policy previews are not its access policy.

### MCP resources and prompts

Add get_memory_health as a read tool: 15 default,16 deletion-enabled,10 read-only.
Resource templates `project-memory://projects/{project_id}/brief` and `/health`
reuse Store operations and all policy/audit/budget protections. No static
archive-wide enumeration. Template visibility respects allowed operations;
direct reads enforce them again. Strict URI parsing; no path/file/network fetch.
Resource response size is bounded separately and never truncates records.

Prompts `resume_project`, `review_change`, `prepare_handoff` take explicit
project_id and optional task (max2000). They return instructions without reading
memory or granting action permissions. Reject unknown arguments. Advertise
resources/prompts through actual MCP capabilities and retain all tool fallbacks.

## Acceptance and limits

Test no-write setup, policy preservation, safe path handling, snapshot/budget
health pagination, no semantic overclaims, revision/freshness UI, stale-response
guards, cross-project policy denial through tools/resources, prompt validation,
and real protocol discovery/read/get. Test only synthetic temporary archives.
Keep previous releases intact. Build frontend/native serially, complete all
platform/security checks, validate actual public artifacts and Registry readback.
Plaintext, local-owner/cursor metadata, audit atomicity and desktop preview
limitations from v0.4 remain in force.
