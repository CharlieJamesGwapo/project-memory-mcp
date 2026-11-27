# Install the Project Memory developer toolkit

The toolkit has one entry skill, `project-memory`, plus the three compatible
focused skills: `resume-project`, `record-decision`, and `prepare-handoff`.
The entry skill loads only the requested workflow: budgeted context, code review,
agent coordination, or security review. You can keep using the old skill names.

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

## Budgeted context and compatibility

The v0.2 route can use `search_memory` for literal cross-kind matches and
`get_context_bundle` for whole records under a byte cap. Default bundle limits
are 16000 bytes and 8 records; accepted ranges are 4096–65536 bytes and 1–20
records. Explicit record-ID selections either fit entirely or return an error.
Automatic selection reports budget/record omissions and its candidate window.

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
