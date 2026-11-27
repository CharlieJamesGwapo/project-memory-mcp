# Resources, prompts and compatible clients

v0.5 adds read-only resource views and explicitly selected workflow prompts.
The existing tools remain available when a client does not expose these features.
Client support varies; setup preview does not test a client's connection or UI.

## Resource views

The server lists templates rather than enumerating archive-wide project resources.
Resolve the exact project UUID through permitted tools, then use:

| URI template | Underlying operation | Scope |
|---|---|---|
| `project-memory://projects/{project_id}/brief` | `get_context_bundle` | Default whole-record context bundle |
| `project-memory://projects/{project_id}/health` | `get_memory_health` | Default first assessment page and snapshot counts |

These are local identifiers, not URLs the server fetches. Only the exact routes
and lowercase UUIDs are accepted; file paths, query strings and unknown routes
are rejected. Discovery filters templates by allowed operations. Direct resource
reads still pass through Store's project/operation policy, read-only and audit
rules; a guessed hidden URI cannot bypass a denial.

Resource contents are JSON text, with the same complete records and source
references as the matching tool result. The brief is a default selection, not
the entire project. Health is a bounded page, not proof of archive safety.
Use the underlying tool for queries, explicit record selection, custom budgets,
an optional current head, or additional pages using the returned cursor.

Each serialized resource result is limited to 65536 bytes without truncating
records. The nested tool budget still describes the canonical tool result,
including its JSON compatibility copy; it does not measure the resource envelope
or model tokens. A resource that cannot fit is rejected with guidance to use
the matching tool and a smaller budget.

Ordinary authorized reads preserve existing records, including sensitive text
that may already be present. Resource reads are not a new redaction boundary.
Use the [credential inspection and policy guide](access-policy.md).

## Workflow prompts

| Prompt | Purpose |
|---|---|
| `resume_project` | Recover bounded attributed context and identify next actions |
| `review_change` | Review the actual change using exact commits and observed checks |
| `prepare_handoff` | Draft completed/unfinished work and actual verification |

All take `project_id` and optional `task` (1–2000 characters when supplied).
Unknown names/arguments, invalid UUIDs and out-of-policy project IDs are refused.
Names use underscores here; installed skill folder names retain their hyphens.

Getting a prompt returns instructions. It does not retrieve memory, execute
checks, run agents or save a record. The client must discover its actual tools,
respect policy and current user authorization, and distinguish historical claims
from current evidence. Selecting a review prompt does not authorize edits or
a saved review. Existing explicit authorization remains valid.

## Protocol and cache behavior

Actual stdio tests cover legacy 2025-11-25 initialization and 2026-07-28 discovery
and inline request metadata. Cacheable inventories/resource reads use private
scope and TTL zero under the newer protocol. Older peers receive the compatible
legacy shape without those hints. No cross-user public caching of project
resource responses is requested. The server does not subscribe to remote feeds.

These tests verify protocol behavior, not every AI client's interface. Use
[setup preview](setup.md), [CLI](cli.md) and [tools](api-reference.md) when a
client lacks resource pickers or prompt selection.
