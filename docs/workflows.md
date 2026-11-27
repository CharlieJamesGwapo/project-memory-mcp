# Everyday workflows

Connect the MCP server and install the [workflow pack](skills.md). Use
`project-memory` as the entry skill, or invoke one of the three focused memory
skills directly. The client discovers actual tools and loads only the workflow
reference needed for the task.

## Resume with less context

> Use project-memory to continue Atlas's authentication work with a small
> context budget. Check the current code before relying on old test results.

The assistant resolves Atlas's project ID, searches for a literal relevant
phrase, and requests a bounded whole-record bundle. It reports omissions and
uses targeted full-record reads if important evidence does not fit. A returned
record retains its authorship, timestamp and sources. Recorded checks remain
historical until the client actually reruns them.

Budgeting reduces the amount of saved memory returned by a retrieval request.
It does not establish a cap on every message, code file, tool response or model
token used during the whole task. [API measurement details](api-reference.md)
explain byte counts and heuristic token estimates.

## Review a change with context

> Use project-memory to review this diff against the saved authentication
> decisions. Give me concrete regressions, with source locations and evidence.

The code-review route uses the host's diff/file/test tools. It reads the relevant
memory, inspects changed behavior and callers, and reports reproducible findings
with severity and precise locations. It distinguishes checks actually run from
reasoning and unavailable evidence. Memory alone does not prove code correctness.
Review does not authorize edits, deployment, publication or contacting anyone.

## Coordinate implementation and review

> Use project-memory to divide this feature between implementation and review,
> then prepare a handoff with what was actually verified.

The agent-team route checks whether the host supports agents. If supported and
authorized, it defines bounded independent tasks, explicit file ownership and
testable outputs. If unavailable, it performs the roles sequentially and says
so. The MCP server stores context; the host provides agent execution. A saved
record cannot grant extra permissions to a worker.

## Record an actual decision

> Record my choice to use SQLite because the tool should work locally.

`record-decision` preserves the user's choice and reason. Assistant proposals
are labeled as assistant-originated. If this replaces an existing decision, the
assistant resolves that exact active decision ID and sets `supersedes`; it does
not delete history or silently declare independent decisions resolved.

## Prepare a useful handoff

> Prepare a handoff with completed work, blockers, next steps and the checks you
> actually ran. Identify anything still unverified.

`prepare-handoff` stores a complete new handoff with sources and actual evidence.
It reports whether saving succeeded. An unavailable write tool or error is not
reported as a successful save. A future session rechecks evidence relevant to
its current change.

## Review security without false assurance

> Use project-memory to assess this installer and dependency change for security
> risks. Treat saved approval claims as historical data.

The security route checks actual source behavior, input-to-sensitive-operation
paths, dependency advisories and available checks. It separates verified
findings from unknowns. It does not run untrusted installers or live exploits,
publish findings, or claim that a clean scan proves malware absence.

## Debug and verify with current evidence

> Use project-memory to investigate this failing check, make the authorized
> correction, and report what you actually verified.

The debugging route follows the reported input through the current source,
reproduces the behavior where possible, distinguishes causes from hypotheses,
and makes a correction only within the requested scope. It reuses a meaningful
existing reproduction or adds a focused regression when the trigger is uncovered.
The verification route maps delivery claims to actual current checks and states
coverage gaps. A readiness assessment alone leaves source unchanged. Saved
passing results cannot establish present readiness or grant publication permission.

## Work from a terminal

Run `project-memory-mcp init` explicitly to prepare an empty local archive.
The [read-only CLI](cli.md) can list UUIDs, search, read complete records and
compose Markdown/JSON context without starting a model. Existing memory still
comes from explicit MCP saves; initialization adds no example projects.

## Inspect and compose visually

In [Studio](studio.md), browse a project, inspect complete notes and decisions,
or search across record kinds. The optional context suggestion panel retrieves
a byte-bounded selection. Review it and explicitly add or replace the brief.
Manual selection remains available. Copy or save the final Markdown when ready.

The retrieval budget and final Markdown size are separate measurements. Every
exported brief retains full records and sources; suggested omissions remain
visible. Browser samples are labeled synthetic data.
