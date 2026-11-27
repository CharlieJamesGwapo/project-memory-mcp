# v0.4.0 security and review design

The user approved building, testing, documenting and publishing this release on
GitHub and the free MCP Registry. This design implements the approved roadmap.

## Decisions and boundaries

Keep Rust 1.100, SQLite schema 1 and immutable full records. Prefer enforcing
policy in Store over prompt-only rules or an external proxy: every public MCP,
CLI and internal Studio operation passes through Store. Local policy is not
authentication against another process with direct file access. No cloud,
shell runner, repository crawler, model API or automatic agent execution.

Policy is an optional, absolute local JSON file supplied by `--policy`. Reject
unknown fields/operations, invalid IDs, symlinks, oversized files and malformed
JSON before opening any archive. Missing policy means all projects/ordinary
operations, deletion disabled, high-confidence secret writes rejected. Policy
may restrict project UUIDs and operations; expose only allowed MCP tools and
enforce the same restriction in Store, including project lists/diagnostics.
`allow_delete:true` explicitly enables deletion; model arguments cannot enable it.
An optional secret mode `allow` supports deliberate local use; never silently
redact immutable records. Credential detection is narrow, deterministic and
returns category/path only. Export inspection reports matches without returning
credential values; normal authorized reads still return complete records.

An optional append-only metadata audit file records operation, project ID,
timestamp, outcome and policy denial, without saved text/query/secret values.
Opening refuses symlinks; writing failure fails closed before any mutation.
Successful/failed operation receipts are best-effort after completion and cannot
promise atomicity with a separate file. Document this boundary clearly.

## Reviews without migration

Two tools, `save_review` and `get_review_history`, store bounded structured
reviews inside immutable note records. Capture repository label, exact base/head
commit SHA, summary, actual checks and bounded findings with IDs, severity,
relative path/line, evidence, correction and status. Revisions explicitly link
an earlier review in the same project; reject unrelated records, forks and
cross-project links. A supplied current head makes stale evidence visible; the
server does not inspect Git or certify user-supplied verification.

Add `get_access_policy` as a read tool. Default tools become 14 (12 prior tools,
minus disabled delete, plus three new); deletion opt-in exposes 15, read-only
exposes nine. Full record retrieval/export remains compatible with schema 1.

## Skills and user flows

Retain four installable skill folders and one progressively loaded entry.
Add focused routes: test planner, dependency review, safe refactor, release
readiness, memory hygiene and architecture review. Strengthen review/security
routes around evidence, exact commits, host capability discovery and actual
authorization. Skills cannot grant tools or attest malware absence.

CLI adds policy-preserving configuration, access policy display and sanitized
archive credential inspection. Existing doctor gains actionable policy context.
Add a separate backup verification helper that restores/checks a temporary copy
without replacing the user's archive. Update setup, API, security, architecture,
data-safety, workflow and release guides, showing exact defaults and limits.

## Acceptance

Tests must demonstrate denied direct calls, restricted project pagination,
read-only enforcement, no data leakage in errors/audit reports, credential
false-positive/negative scope, deletion opt-in, immutable review revisions,
cross-project/fork rejection, stale-head reporting and old archive compatibility.
Use only synthetic archives. Exercise actual stdio and published MCPB. Verify
all platform CI, package inventories, checksum/signature provenance and exact
Registry metadata before reporting publication. Preserve existing releases.
