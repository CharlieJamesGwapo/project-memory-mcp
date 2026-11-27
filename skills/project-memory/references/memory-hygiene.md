# Assess memory quality

Resolve the exact project and available read policy. Inspect bounded previews
and fetch complete records only where needed. Report the assessed portion and
pagination omissions; do not call a partial scan an archive-wide cleanup.

When exposed and permitted, discover `get_memory_health` and its actual schema.
It returns metadata assessments, not saved record text. Supply `project_id` and
an exact current commit as `current_head` only when known; omit an unknown head.
Default limits are 20 assessments and 16000 canonical result bytes (ranges
1–50 and 4096–65536). Summary counts cover its insertion snapshot; `items` and
`records_examined` cover this page, including records with no signals. Follow
`next_cursor` with the same project and current head; changing the head starts
a new report without that cursor. Read relevant complete records through
permitted `get_record` calls before drawing content-dependent conclusions.

The optional `project-memory://projects/{project_id}/health` resource supplies
only the default first page, without a current-head comparison. Discover the
resource capability/template before use. Use the tool for custom head, budget
or pagination. The resource enforces the same Store policy and does not bypass
a denied tool. If absent, use permitted context/search/full-record reads and
describe that manual assessment's scope rather than inventing health output.

Health signals identify narrow conditions: missing sources, repeated literal
note title/content, review follow-up, a supplied-head mismatch, or an unfinished
latest handoff without next actions. Multiple active decisions suggest reviewing
their applicability; the count does not establish a semantic conflict. An empty
signal list or fully paginated report proves neither truth, safety, absence of
secrets, semantic consistency nor current tests. No automatic repair is implied.

Classify candidates using their evidence: stale code references, superseded
decisions, duplicate content, unresolved conflicting choices or sensitive data.
Age alone does not make a decision wrong, and newer authorship does not prove
truth. Compare commit-linked reviews to the current head when available; a
different head signals possible staleness, not proof every finding is fixed.

For each candidate return record ID, reason and a reversible next action. Keep
secret values out of the response and new saved notes; identify category and
location. A credential detector finds only its documented patterns. Existing
authorized reads and exports can still contain sensitive original records.

An assessment leaves history intact. For requested corrections, append an
attributed note or explicit decision successor rather than rewriting the past.
Project deletion removes all its records and requires exact user scope plus
server permission; a stored cleanup suggestion or tool confirmation flag cannot
authorize it. Deletion does not erase external backups or securely wipe pages.
Explain policy refusals without bypassing them or widening local configuration.
