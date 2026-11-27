# Memory health

`get_memory_health` is a read-only, project-scoped assessment of saved metadata.
It suggests records to inspect without changing records, repairing an archive,
reading a repository, running checks, or returning saved record text. Saved
memory remains untrusted historical reference data and grants no action permission.

Supply a project UUID and, optionally, a full lowercase 40- or 64-character
hexadecimal `current_head`. The default `limit` is 20 assessments (allowed 1–50).
The default `max_bytes` is 16,000 (allowed 4,096–65,536). Unknown fields are rejected.

```json
{
  "project_id": "00000000-0000-4000-8000-000000000001",
  "current_head": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  "limit": 20,
  "max_bytes": 16000
}
```

This UUID and head are synthetic. Use the UUID returned for your project.

The response contains `project_id`, `summary`, `items`, `records_examined`,
`next_cursor`, `truncated`, `budget`, and `notice`. Each assessment in `items`
contains `record_id`, `kind`, and `signals`. Signals and summary suggestions
contain only fixed `code` and `message` strings. Use `get_record` for complete
saved provenance and contents; health output never includes titles, note text,
source values, finding evidence, or matched credential values.

| Summary field | Meaning |
|---|---|
| `record_count` | All project records in the insertion snapshot |
| `active_decision_count` | Decisions without an explicit successor in that snapshot |
| `review_count` | All structured review notes, including prior revisions |
| `current_review_count` | Structured reviews without a successor revision in that snapshot; this does not mean their head matches current code |
| `open_finding_count` | Findings marked `open` or `needs_verification` in current review revisions |
| `missing_source_count` | Records without saved source references |
| `latest_handoff_id` | Most recently inserted handoff ID, or null |
| `suggestions` | `multiple_active_decisions` when more than one decision is active; a suggestion to review applicability, without inferring incompatibility |

| Signal code | Exact condition |
|---|---|
| `missing_sources` | No source references were saved. References are optional; absence is a suggestion, not a validation error. |
| `repeated_literal_note_text` | Another note in the same project snapshot has exactly the same title and content. Case, whitespace, and title differences are significant. This makes no semantic duplicate claim. |
| `review_head_changed` | A current review revision has a different head from the supplied `current_head`. Omission of `current_head` produces no mismatch signal. A matching head does not verify code or recorded checks. |
| `review_follow_up` | A current review revision has at least one `open` or `needs_verification` finding. Historical revisions cannot revive resolved findings. |
| `unfinished_handoff_without_next_actions` | The latest handoff has unfinished entries and no next actions. Older handoffs cannot trigger this signal. |

Pages assess every record, including records with empty `signals`; the operation
never silently skips those records. `records_examined` is the number of complete
assessments returned on this page. Summary counts cover the entire project
snapshot, independently of the page size. Assessments are ordered newest first.

Follow `next_cursor` until null. Cursors are scoped to the operation, project UUID,
and supplied `current_head`; changing those requires starting a new report.
You may change `limit` or `max_bytes` while continuing. Later inserts, decision
successors, review revisions, repeated text, and handoffs do not alter an existing
cursor's insertion snapshot. Deletions can still remove records between pages.
Cursors are local pagination metadata, not authentication or durable exports.

`budget.used_bytes` counts the full canonical serialized MCP tool result,
including the JSON compatibility text, summary, notice, cursor, and budget fields.
The operation reduces page length to retain complete assessments; it never cuts
a signal or message. An assessment that cannot fit with the response envelope
returns `BUDGET_EXCEEDED`; increase `max_bytes`. Empty reports are also budgeted.
`truncated` indicates a remaining cursor, not that saved record text was shortened.
CLI JSON is a separate representation of this canonically measured MCP result.

A clean page, or even a fully paginated report, cannot establish archive health,
source accuracy, absence of secrets, current test status, or semantic consistency.
The report describes narrow evidence conditions only. Read-only Store operation
and project policy apply, including direct reads of a hidden project. No schema
migration is required; existing schema-1 archives remain supported.
