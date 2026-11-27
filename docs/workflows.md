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

If advertised by the host, the `resume_project` prompt can supply these workflow
instructions using the resolved `project_id` and optional `task`. It reads no
memory itself. A discovered `/brief` resource is a default bounded first bundle;
use `get_context_bundle` for a task query or custom limits. Discover actual
capabilities and schemas; permitted tool fallbacks remain usable on older hosts.

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

For a requested saved review on v0.4, the assistant discovers the actual
`save_review` schema and records exact base/head commits, attributed evidence,
checks and finding statuses. `get_review_history` can compare the recorded head
with a supplied current head. A different hash makes the review potentially
stale; it does not establish that the findings were fixed. Ordinary notes are
an explicitly labeled fallback on older servers. See [code review](code-review.md).

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

The v0.4 access-policy read tool describes the connection's actual restrictions.
The assistant respects project/operation limits and read-only mode. A denied
write does not authorize modifying policy or accessing the archive through
another route. Narrow credential detection and metadata audit records provide
scoped checks, not universal secret detection or proof of human consent.

## Choose tests and assess dependency changes

> Use project-memory to plan meaningful regression checks for this change, then
> review the dependency upgrade against our supported platforms.

Test planning maps each material behavior to an observable check and identifies
existing coverage. Dependency review distinguishes active runtime/build paths
from optional lockfile entries, checks current version-specific evidence and
reports compatibility or reachability limits. Neither planning nor review alone
updates source or dependency locks. If implementation is requested, the assistant
executes the authorized checks through its host and reports actual outcomes.

## Refactor and review architecture

> Use project-memory to simplify this module while preserving its public API,
> errors and authorization behavior.

The safe-refactor route identifies the stable contract and makes the smallest
useful change with relevant regression coverage. Architecture review assesses
actual entry points, state ownership and trust boundaries against stated goals.
It compares viable alternatives without assuming a larger system is required.
A design assessment alone leaves implementation unchanged.

## Assess memory quality and release readiness

> Use project-memory to identify stale review evidence and assess this release.
> Leave history intact and tell me which checks are still missing.

Memory hygiene returns record IDs, evidence-based reasons and reversible next
actions for the portion actually inspected. It does not equate age with error,
delete history automatically, or print sensitive original values. Release
readiness ties the assessment to an exact source/version and actual artifacts,
checksums, provenance and relevant platform checks. Source tests, successful CI
and interactive OS verification remain separate evidence. Publication occurs
only within the user's actual authorization and is followed by remote readback.

On v0.5, discover `get_memory_health` when available. Its summary counts cover
the insertion snapshot while `items` cover a bounded page, including records
without signals. Supply a known exact `current_head` for review-head comparison.
Continue a cursor with the same project and head; a changed head starts a new
report. Retrieve complete candidate records before judging their content. The
optional `project-memory://projects/{project_id}/health` resource supplies only
the default first page without a supplied head; use the tool for pagination or
custom inputs. Resources enforce the same Store policy as tools.

Health identifies narrow metadata conditions, not semantic conflicts, truth,
safety, absence of secrets or current check results. Multiple active decisions
suggest reviewing applicability; they can be compatible. Neither a clean page
nor a fully paginated report establishes those broader claims. These views
never repair records or authorize saving. When absent, use permitted existing
read tools and report the manual assessment's actual scope.

The optional `review_change` and `prepare_handoff` prompts also return only
instructions. Each requires `project_id` and accepts optional `task` up to 2000
characters. Selecting one does not retrieve evidence or grant write permission.
Use the corresponding skill workflow directly when prompts are unavailable.

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

The [setup preview](setup.md) helps prepare explicit paths and stdio JSON without
initializing an archive, appending audit entries or writing client settings. It
does not execute the server or test a client connection. SQLite inspection can
still maintain WAL/SHM coordination files; read-only describes memory and schema.
Installing skills is a separate step. Protocol tests cover real modern/legacy
SDK paths, not every host's resource or prompt UI.

## Inspect and compose visually

In [Studio](studio.md), browse a project, inspect complete notes and decisions,
or search across record kinds. The optional context suggestion panel retrieves
a byte-bounded selection. Review it and explicitly add or replace the brief.
Manual selection remains available. Copy or save the final Markdown when ready.

The retrieval budget and final Markdown size are separate measurements. Every
exported brief retains full records and sources; suggested omissions remain
visible. Browser samples are labeled synthetic data.
