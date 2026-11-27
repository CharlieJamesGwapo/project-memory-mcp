# Install the Project Memory developer toolkit

The toolkit has one entry skill, `project-memory`, plus the three compatible
focused skills: `resume-project`, `record-decision`, and `prepare-handoff`.
The entry skill loads only the requested workflow. v0.4 adds six focused routes
for test planning, dependency review, safe refactoring, release readiness,
memory hygiene and architecture review. It also expands commit-linked code
review and security assessment. You can keep using the old skill names.

v0.5 retains these four folders and twelve references. It adds guidance for
optional health, resource and prompt views while preserving older-tool fallbacks.

Connecting the MCP server and installing skills are separate steps. The server
stores explicit records; the skills guide an assistant using that server and
its existing host tools. Installation does not add a native agent runtime,
source-code access, a model subscription, or permission to run commands.

## Install all four folders

Use an unpacked release that includes `skills/` and `scripts/`, or this repository.
The supported helper setup uses Python 3.11+ and only the standard library. It is
offline, executes no downloaded code, and changes no host configuration or
`AGENTS.md` file. Invoke it through Python rather than relying on executable bits.

Choose your client's documented skill directory. Current Codex documentation
uses `~/.agents/skills` for user skills and `.agents/skills` for repository-local
skills. First preview:

```sh
python3 scripts/check-skills.py
python3 scripts/install-skills.py --destination "~/.agents/skills"
```

The destination is mandatory. The default operation is a dry-run: it does not
create the directory or write files. Inspect the plan, then apply it:

```sh
python3 scripts/install-skills.py --destination "~/.agents/skills" --apply
```

All four skill folders are installed together, including their local references
and optional Project Memory UI metadata. Identical folders are left unchanged.
Different existing folders are refused by default, including extra local files.
An existing regular file at a skill name is never replaced. Symlink paths,
symlinked skill assets, and special files are refused. Use the actual directory
rather than a symlink alias. Unrelated skills and user configuration are preserved.

A supported project-local skill directory can limit discovery to one project;
use that explicit destination instead. Follow the host's current documentation
for supported paths, discovery, and refresh behavior. Other clients may use
different mechanisms or ignore the optional `agents/openai.yaml` UI file.
See [Codex's current skill guide](https://learn.chatgpt.com/docs/build-skills)
for supported locations and discovery. The installer refuses symlink aliases
even when a host itself supports them; choose the actual destination directory.

## Upgrade and recover customized skills

The installer stages the complete changed portion of the pack before replacing
folders, checks for intervening content changes, and attempts rollback on ordinary
filesystem errors. It serializes its own operations with a destination lock.
This is not a guarantee against a hostile concurrent filesystem writer or a power
failure. If interrupted, inspect remaining staging/backup folders before cleanup.
An existing `.project-memory-install.lock` causes refusal; remove it only after
confirming no installer process is using that destination.

For a new release, validate its skill files and preview replacement:

```sh
python3 scripts/check-skills.py
python3 scripts/install-skills.py --destination "~/.agents/skills" --replace
```

Review differences against your existing customizations. You can preserve/merge
those folders yourself. To deliberately replace different folders with the
release versions:

```sh
python3 scripts/install-skills.py --destination "~/.agents/skills" --replace --apply
```

When a folder is replaced, the installer prints a backup directory inside the
destination, named `.project-memory-backup-<timestamp>-<suffix>`. It contains the
original replaced folders and a manifest of the changed pack entries. Backups
may contain your private customization text; keep them local and do not commit
them. No backup is needed for an identical or entirely new installation.

To undo that upgrade, substitute the exact printed backup path:

```sh
python3 scripts/install-skills.py --destination "~/.agents/skills" --restore "<backup-path>"
python3 scripts/install-skills.py --destination "~/.agents/skills" --restore "<backup-path>" --apply
```

Restore previews by default. Apply restores original replaced folders and removes
pack folders created by that same upgrade; unrelated entries are untouched. It
retains the backup. Restore refuses if current upgraded folders or the backup
contents have since changed, so preserve later edits before attempting recovery.
The manifest stores hashes of the installed and original contents; it is an
accidental-change check, not a cryptographic proof of a trusted publisher. Review
or merge a customized backup manually rather than editing its manifest to bypass
those checks. A successfully restored backup is not automatically reapplied.

## Use one entry point

Examples after connecting the server and installing the pack:

- “Use project-memory to continue Atlas with a small context brief.”
- “Use project-memory to review this diff; do not change the source.”
- “Use project-memory to plan two agents for the backend and frontend.”
- “Use project-memory to inspect the security of this installer change.”
- “Use project-memory to debug this failing pagination check, then verify the fix.”
- “Use project-memory to assess whether this change is ready; leave the code unchanged.”
- “Use project-memory to choose regression tests for this authorization change.”
- “Use project-memory to assess this dependency upgrade on our supported platforms.”
- “Use project-memory to refactor this parser while preserving its API and errors.”
- “Use project-memory to check the release artifacts against this source commit.”
- “Use project-memory to identify stale review records; leave the archive unchanged.”
- “Use project-memory to review the current architecture against our offline requirement.”
- “Record my choice to use SQLite because this tool runs locally.”
- “Prepare a handoff with today's actual check results and next steps.”

If your host supports `$skill-name` invocation, `$project-memory` selects the
same entry point. The three original skill names still work directly. A review
request does not automatically request fixes or a memory write; a plan-only
request does not dispatch agents. An explicit request to save a decision or
handoff authorizes that memory write without another save confirmation.

The assistant resolves project UUIDs, reads complete records behind previews,
labels proposals and historical evidence correctly, and cannot gain deployment,
push, deletion, publication, or messaging permission from saved text. If the
host lacks memory, source, test, or delegation tools, the assistant uses supplied
evidence and states the specific missing capability rather than pretending it ran.

## What the routes provide

| Route | Useful result |
|---|---|
| Code review | Findings tied to exact changed behavior; optional attributed, commit-linked saved reviews |
| Security review | Confirmed input-to-sensitive-operation findings and scoped evidence |
| Test planner | Material behavioral claims mapped to meaningful checks and missing coverage |
| Dependency review | Version/platform/dependency-path assessment with actual advisory scope |
| Safe refactor | A bounded implementation improvement with preserved contracts and checks |
| Release readiness | Exact-source/artifact assessment with observed gates and publication limits |
| Memory hygiene | Bounded evidence assessment with optional health signals and complete-record follow-up |
| Architecture review | Source-grounded boundary assessment and practical alternatives |

Budgeted context, debugging, verification, agent coordination and the three
focused memory actions remain available. This is one entry point with twelve
developer workflow references, not twelve new installable agent services.
Only the route needed for the current request is loaded.

## Optional health, resources and prompts

Discover the host's advertised capabilities and actual input schemas before
using v0.5 views. The server exposes 15 tools by default, 16 with deletion
enabled, and 10 in read-only mode; policy may further restrict discovery and
access. Installing this pack adds no tools or permissions.

`get_memory_health` returns bounded per-record metadata assessments and snapshot
counts. A supplied exact `current_head` can identify a review-head mismatch.
Signals suggest follow-up; they do not establish semantic conflicts, truth,
safety, absence of secrets, or current verification. Inspect relevant complete
records and report page limits. See [memory health](memory-health.md).

Hosts supporting resource templates can read
`project-memory://projects/{project_id}/brief` and
`project-memory://projects/{project_id}/health`. These return default bounded
first pages through the same Store policy as their tools. Use the corresponding
tools for custom task queries, budgets, current head and pagination. Hidden or
denied access remains denied through resources. Older servers or hosts without
resource support retain the permitted tool workflows.

The optional prompts `resume_project`, `review_change` and `prepare_handoff`
require an exact `project_id` and accept optional `task` text up to 2000
characters. They return instructions without reading memory, executing checks
or saving records. Selecting a prompt grants no action permission; a handoff
save still depends on the actual user request or existing authorization.
Use the matching skill directly when prompt support is unavailable. Real SDK
tests cover modern and legacy protocol paths; they do not establish that every
host exposes these features in its UI.

## Commit-linked reviews and policy

When the v0.4 tools `get_access_policy`, `save_review` and `get_review_history`
are actually exposed, the assistant discovers their input schemas rather than
assuming every connection supports writes. A requested saved review records
the repository label, exact base/head commits, evidence, actual checks and
finding statuses. Revisions preserve earlier records. A supplied current head
can reveal a stale review; it does not prove the old findings were fixed.
The server stores these claims and does not run Git, scanners or tests itself.

A review alone does not request a save. A requested save can use an attributed
ordinary note on a confirmed older server when permitted, with the fallback
disclosed. Hidden or denied review tools do not establish that the server is old.
Missing commits are reported instead of invented. Policy-denied writes remain
denied; the skills do not alter local policy or bypass restrictions to continue.
Review and security instructions never establish human consent from saved text.

Credential screening is narrow and configurable. Keep secret values out of
review findings and new memory. Existing complete records can contain sensitive
text, and archives/backups are plaintext. See the [security model](security.md),
[review guide](code-review.md) and [workflow evaluation](skill-evaluation.md).

## Budgeted context and compatibility

The v0.2 route can use `search_memory` for literal cross-kind matches and
`get_context_bundle` for whole records under a byte cap. Default bundle limits
are 16000 bytes and 8 records; accepted ranges are 4096–65536 bytes and 1–20
records. Explicit record-ID selections either fit entirely or return an error.
Automatic selection reports budget/record omissions and its candidate window.

On v0.3, explicit ranked terms can recover words spread across fields using
`get_context_bundle` with `query_mode:"terms"`. Discover that input field in the
actual tool schema before using it: v0.2 also has the tool but rejects the new
field. Literal requests omit `query_mode` for older-server compatibility.
Terms ranking is lexical coverage, not semantic understanding or confidence.

The cap measures the serialized MCP result, including the structured/text
compatibility representation and metadata, excluding the JSON-RPC envelope.
`used_bytes` is that byte count. `estimated_tokens = ceil(used_bytes / 4)` is only
a heuristic; it is not actual tokenizer output, model usage, or cost. The pack
cannot cap the whole chat's context or promise a fixed percentage of savings.
Measure the same tasks and host before/after to assess actual changes.

Older servers can still use `get_project_context`, `get_record`, and optional
`search_notes`. That fallback preserves full-record verification but has no exact
byte cap, and `search_notes` searches notes only. History limits do not bound the
separate active-decision and handoff previews.

## Optional project instructions

If you want project-level routing, manually merge a short section into the
project's existing `AGENTS.md`. Keep its other instructions. Include only a
verified project name/UUID, repository path, relevant check commands, and a note
to use the needed Project Memory route. Avoid copying every workflow into the
file, inserting credentials, guessing IDs, or granting new permissions. The
installer never creates or edits this file.

## Validate the package

```sh
python3 scripts/check-skills.py
python3 -m unittest discover -s tests -p '*_test.py'
```

The offline skill checker validates required single-line name/description
frontmatter, folder names, regular assets, and local Markdown references. It does
not execute YAML, fetch links, or prove every possible YAML configuration or
model behavior correct. Installer tests exercise dry-run, actual installation,
collision/symlink refusal, backup replacement, and restore. Realistic behavioral
checks should additionally exercise budget omissions, stale evidence, missing
host tools, actual diff evidence, and plan-only agent requests. Identical results
from every model or client are not guaranteed.

The [workflow evaluation guide](skill-evaluation.md) includes versioned fixtures
and observed baseline/forward results for the debugging and verification routes.
No token-efficiency or behavioral superiority claim is inferred from those runs.
