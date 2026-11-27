---
name: record-decision
description: Use when a user asks to save a project decision or explicitly changes a decision previously recorded in Project Memory MCP.
---

# Record a project decision

Capture the choice, its reason, and who made it. A question, option under
discussion, or assistant recommendation is not a user's decision.

## Resolve the context

Discover `record_decision`, `get_project_context`, and `get_record`; host
prefixes may vary. Resolve the exact project UUID with `list_projects` when
needed. Ask about an ambiguous project rather than writing into a guessed one.
If the MCP connection is unavailable, provide a draft and say it is unsaved.

If exposed, inspect `get_access_policy`; respect read-only and project/operation
restrictions. A denied save does not authorize changing policy or using another
archive connection. Keep credential values out of decisions and sources; a
credential-screening error is not a reason to disable the guard automatically.

For a change to an existing choice, read the active decision previews from
`get_project_context`, then retrieve relevant full records with `get_record`.
If available, `search_memory` with `kind: "decision"` can locate literal matches;
its results remain previews. `get_context_bundle` can retrieve complete known
records, so those records need no duplicate fetch. Read the bundled
[retrieval contract](../project-memory/references/token-saver.md) when using
these optional tools; retain the existing read-tool fallback on older servers.
Follow history cursors when needed. A matching keyword does not prove that two
decisions address the same subject.

## Save the actual choice

Call `record_decision` with:

- `project_id`: the resolved project UUID.
- `decision`: the explicit choice, max 8000 characters.
- `rationale`: the stated reason, max 8000 characters. If no reason was given,
  record “Reason not provided” rather than inventing one.
- `author_type`: `user` for an explicit user choice; `assistant` for an
  assistant proposal the user specifically asked to save as a proposal.
- `sources`: up to 10 concrete references, each max 2048 characters. An empty
  array is valid when no durable reference exists. References are not proof of
  independent verification.
- `supersedes`: the prior active decision's UUID only when this choice
  explicitly replaces it in the same project. Otherwise omit it.

Total input is limited to 65536 UTF-8 bytes. Unrelated decisions remain active.
The server preserves prior records; this skill does not delete history.

On `CONFLICT`, reload the prior decision and explain its current successor.
Do not silently retry as a new independent decision or guess a replacement ID.
Confirm a save only after a successful tool result; include the returned record
ID when useful.

Example: “Switch Atlas from SQLite to Postgres because we now need concurrent
team writes” replaces Atlas's active database decision. “Would Postgres be
better?” calls for discussion and leaves the existing decision unchanged.
