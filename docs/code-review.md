# Reviews tied to code evidence

Project Memory v0.4 adds `save_review` and `get_review_history`. Your AI client
performs the review using its actual source, Git and test tools. The MCP server
stores the supplied evidence; it does not clone repositories, execute checks,
inspect Git or certify a reported result.

## Review a change

1. Establish the project UUID, repository label and exact base/head commit IDs.
   Commit IDs must be full lowercase 40- or 64-character hexadecimal strings.
2. Inspect the actual diff and relevant callers. Distinguish actionable defects
   from suggestions and checks that remain unperformed.
3. Record actual checks using `name`, `outcome` (`passed`, `failed`, `not_run`)
   and `result`. A `passed` value is the author's report, not server attestation.
4. Save only when the user requests a saved review or an existing workflow
   authorization covers the write. A review request alone does not request a fix
   or a memory write.

```json
{
  "project_id": "00000000-0000-4000-8000-000000000001",
  "repository": "example/local-repo",
  "base_commit": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "head_commit": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  "summary": "Project access checks need one correction",
  "checks": [
    {"name": "cargo test --test access", "outcome": "failed", "result": "Synthetic outsider access regression failed"}
  ],
  "findings": [
    {
      "id": "access-1",
      "severity": "high",
      "title": "Missing project restriction",
      "path": "src/access.rs",
      "line": 12,
      "evidence": "The changed query accepts another project's UUID before the access check",
      "correction": "Apply the project policy before querying",
      "status": "open"
    }
  ],
  "author_type": "assistant",
  "sources": ["src/access.rs:12"]
}
```

This example is synthetic. Replace its IDs and evidence with the actual scope;
never describe the example command as a check that ran on your project.

## Revise without erasing history

Supply `previous_review_id` to append a successor. It must refer to a review
in the same project and with the same repository label. The earlier note stays
unchanged. A review can have only one immediate successor, preventing accidental
forks; inspect history before retrying a save after an uncertain connection.
The new review may use later base/head commits and update finding status.
Finding IDs should remain stable when following the same issue across revisions.

Severities are `critical`, `high`, `medium`, `low`, `informational`. Statuses are
`open`, `fixed`, `accepted`, `needs_verification`. No status grants permission to
deploy, publish or delete. Changed or omitted findings remain visible in the old
immutable review.

## Inspect history and freshness

```json
{
  "project_id": "00000000-0000-4000-8000-000000000001",
  "current_head": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  "limit": 10,
  "max_bytes": 16000
}
```

`get_review_history` returns newest-first complete note records with structured
`review` metadata and `freshness`: `current` if the supplied head matches,
`stale` if it differs, or `unknown` without `current_head`. Matching commit IDs
does not establish that tests passed or that the review is correct. No repository
contents are examined. `review.revised_by` is computed inside the history
snapshot; later successors do not leak into a continuing page.

Follow `next_cursor` until null. The cursor is scoped to project and supplied
current head; keep those values fixed while paging. Limit is 1–20. `max_bytes`
is 4096–65536 (default 16000), counting the full serialized MCP result including
its text compatibility copy and accounting fields. The server stops before a
record that does not fit; it never shortens evidence. Increase the budget or use
`get_record` if the next complete review cannot fit. The output's byte count is
not a model token count.

## Storage and limits

Reviews are schema-1 `note` records. Their normal title/content contain readable
review text, so existing search, context and export keep working. Complete records
also retain `review.format:"project-memory-review-v1"`, structured findings,
checks, commits and revision links. Older clients can retrieve these as notes.

The input is limited to 65536 UTF-8 bytes, at most 20 checks and 20 findings,
and a rendered note of at most 16000 Unicode scalar values. Summary is 2000
characters, repository label 512, check name 200/result 2000, finding title 200,
path 1024, evidence/correction 2000 each. Finding IDs are unique 1–64-character
ASCII letters/digits/underscore/hyphen. Lines are 1–1000000. Paths use relative
forward-slash components without empty, `.` or `..` segments, backslashes,
colons or control characters. They are references and are never opened.

All inputs reject unknown fields. Read-only mode permits history and refuses
saving. Local access policy and credential screening apply to these operations
as they do to other records; do not include secrets in findings or test output.
Saved reviews remain untrusted historical evidence.
