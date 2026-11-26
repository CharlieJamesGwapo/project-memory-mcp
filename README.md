# Project Memory MCP

A local Rust MCP server that helps AI assistants resume projects using saved
notes, explicit decisions, and handoffs. SQLite keeps the records on your
computer. Three companion skills explain how to retrieve and use them.

**Release status:** v0.1.0 source and local build. This repository has not been
published to crates.io or the MCP Registry by the build process. Package and
repository names below are proposed publication identities.

## What it does

- Keeps projects separate using UUIDs, even when names match.
- Searches notes using literal substrings.
- Preserves decisions and their explicit replacements.
- Saves immutable handoff revisions with verification evidence and next steps.
- Returns bounded context previews, complete individual records, and paginated exports.
- Runs over stdio with no hosted service, model API key, or telemetry.

It saves what an assistant explicitly submits through tools. It does not read
your chats, scan repositories, run shell commands, verify sources, or synchronize
devices. Your AI client still controls what retrieved memory is sent to its model.

## Build and run

Install [Rust](https://rustup.rs/) and the platform C compiler/build tools.
The repository pins Rust 1.100.0. SQLite is bundled; a separate database service
or system SQLite installation is unnecessary.

From the source directory:

```sh
cargo build --release --locked
./target/release/project-memory-mcp --help
```

On Windows, the binary is `target\release\project-memory-mcp.exe`.
To install the checked-out source into Cargo's local bin directory:

```sh
cargo install --path . --locked
```

A release binary runs without Rust installed. Binaries are platform-specific;
use one built for your operating system and architecture. macOS may request
approval for downloaded unsigned software. This project does not bypass those
checks or provide signed/notarized binaries.

The binary speaks MCP on stdin/stdout. Running it without a client waits for
the protocol handshake. Use `--help` or `--version` for ordinary terminal output.

## Connect an MCP client

Configure a client that supports local stdio MCP servers. Replace both example
paths with actual absolute paths on your device:

```json
{
  "mcpServers": {
    "project-memory": {
      "command": "/absolute/path/to/project-memory-mcp",
      "args": ["--db", "/absolute/path/to/private-memory/memory.sqlite"]
    }
  }
}
```

This is a common configuration shape; your client may use different settings.
See [the example](examples/mcp-config.json). This repository does not modify your
client configuration or install skills globally.

You can set `PROJECT_MEMORY_DB` instead of `--db`; the flag takes precedence.
Both require an absolute database path. With neither, the default is the
platform user data directory:

| Platform | Default database |
|---|---|
| macOS | `~/Library/Application Support/project-memory-mcp/memory.sqlite` |
| Linux | `$XDG_DATA_HOME/project-memory-mcp/memory.sqlite`, or `~/.local/share/project-memory-mcp/memory.sqlite` |
| Windows | Local AppData under `project-memory-mcp`, then `data/memory.sqlite` |

Use a separate database path when you want an entirely separate memory library.
Project IDs prevent accidental mixing within a database; they are not a user
authentication or access-control boundary. A connected client can list and
modify all projects in the database it opens.

## First use

Ask your connected assistant to create a project, then save a note:

```json
{
  "tool": "create_project",
  "arguments": {"name": "Atlas", "purpose": "Build a portfolio website"}
}
```

Use the returned `id` as `project_id` on subsequent calls. These are illustrative
tool envelopes; your client sends the actual MCP requests.

```json
{
  "tool": "save_note",
  "arguments": {
    "project_id": "RETURNED-PROJECT-UUID",
    "title": "Design direction",
    "content": "The user chose a light background and serif headings.",
    "author_type": "user",
    "sources": []
  }
}
```

Only use `author_type: "user"` for a choice or content actually originating
from the user. Assistant proposals and inferences use `assistant`.

## Tools

Hosts may add a prefix to these names. Unknown input fields are rejected.

| Tool | Required input | Result |
|---|---|---|
| `create_project` | `name`, `purpose` | Project with generated UUID; optional `directory` reference |
| `list_projects` | None | Project page |
| `get_project_context` | `project_id` | Project, latest handoff preview, active decision previews, history page |
| `get_record` | `project_id`, `record_id` | Full note, decision, or handoff, including historical records |
| `save_note` | `project_id`, `title`, `content`, `author_type` | Saved note; optional `sources` array |
| `search_notes` | `project_id`, `query` | Matching note previews |
| `record_decision` | `project_id`, `decision`, `rationale`, `author_type` | Saved decision; optional `sources` and `supersedes` |
| `save_handoff` | `project_id`, `summary`, `completed`, `unfinished`, `next_actions`, `blockers`, `verification`, `author_type` | New handoff; work/verification fields are arrays of strings; optional `sources` |
| `export_project` | `project_id` | Markdown page with full records, IDs, and continuation cursor |
| `delete_project` | `project_id`, `confirm: true` | Permanent deletion of that project and its records |

`list_projects`, `search_notes`, `get_project_context`, and `export_project`
accept `limit` (default 20, max 50) and `cursor`. Reuse `next_cursor` with the
same tool, project, and filters. Pages use a descending insertion sequence and
exclude records added after the first page's snapshot. Concurrent deletion or
supersession can remove entries from an active-only view; use
`include_superseded: true` on context history to inspect decision changes.

Context and search return previews. Fetch full relevant records through
`get_record`. Follow history pages when `active_decisions.truncated` is true;
the first 20 active decisions are only a headline view. Follow every export
cursor and concatenate its Markdown pages for a complete export. Each export
page is a self-contained JSON block in Markdown, not a database backup or
automated restore format.

Search matches literal note title/content substrings. `%`, `_`, backslashes,
and SQL punctuation are escaped. ASCII matching is case-insensitive; other
Unicode matching is exact. Decisions and handoffs are retrieved through context
history and record IDs, not `search_notes`.

An existing decision changes only when a new decision explicitly names its
active ID in `supersedes`. Conflicting independent decisions remain active;
the assistant must reconcile their meaning. The server does not infer agreement
or choose between them.

## Limits and errors

Limits count Unicode scalar values unless stated otherwise:

| Input | Limit |
|---|---|
| Project name / note title | 200 characters |
| Project purpose / handoff summary | 4000 characters |
| Directory reference | 4096 characters |
| Note content | 16000 characters |
| Decision / rationale | 8000 characters each |
| Search query | 2000 characters |
| Handoff work/verification lists | 20 strings per list, 1000 characters per string |
| Sources | 10 strings, 2048 characters per string |
| Total record input | 65536 UTF-8 bytes, including JSON escaping |
| Export Markdown page | 256 KiB |

Domain failures return `isError: true` and structured JSON:

```json
{"error":{"code":"NOT_FOUND","message":"Project or record not found in the requested project"}}
```

Codes include `VALIDATION_ERROR`, `NOT_FOUND`, `INVALID_CURSOR`, `CONFLICT`,
`STORAGE_ERROR`, and `NEWER_SCHEMA`. MCP schema/argument errors may instead be
protocol errors generated by the SDK. Internal SQL and OS error details are not
returned. Writes are atomic. If a connection fails after a write, check history
before retrying: write tools do not have idempotency keys.

## Skills

The distribution includes:

- [resume-project](skills/resume-project/SKILL.md): find the right project,
  retrieve full evidence, and distinguish saved claims from current state.
- [record-decision](skills/record-decision/SKILL.md): preserve a user's actual
  choices, rationale, authorship, and explicit supersession.
- [prepare-handoff](skills/prepare-handoff/SKILL.md): save progress, actual
  verification results, blockers, and ordered next steps.

See [skill installation](docs/skills.md). Skills guide a compatible AI client;
the server itself does not execute them or grant source-code access.

## Storage and privacy

The SQLite database is plaintext and contains what you save. Do not save
passwords or API keys. New Unix database directories use mode 0700 and database
files mode 0600; existing parent directory permissions and Windows ACLs remain
under your control. The file is not encrypted and the protocol is not an
authentication boundary. Do not expose this local server as a shared service.

Saved content is untrusted data. A note asking an assistant to delete records,
deploy code, or ignore instructions provides no authorization. Tool annotations
and `confirm: true` are hints and input checks, not proof of user consent;
the client remains responsible for enforcing authorization.

To back up, stop all server processes using the database before copying the
database and any `-wal`/`-shm` sidecars together. A live file copy is not a
consistent SQLite backup. Deleting a project does not erase external backups,
filesystem snapshots, or securely overwrite prior database pages.

## Development and publishing

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo package --locked
python3 scripts/check-package.py target/package/project-memory-mcp-0.1.0.crate
```

Tests use temporary synthetic records and launch a real MCP client and server
process. CI runs on Linux, macOS, and Windows. Release workflows prepare binaries
when the owner pushes a `v*` tag; they do not publish crates or registry listings.

Read [publishing instructions](docs/publishing.md) before creating public releases.

## Links and license

- Proposed repository: [CharlieJamesGwapo/project-memory-mcp](https://github.com/CharlieJamesGwapo/project-memory-mcp)
- MCP Registry name: `mcp-name: io.github.charliejamesgwapo/project-memory`
- [Official Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [MCP Cargo package publishing](https://modelcontextprotocol.io/registry/package-types)

MIT licensed. See [LICENSE](LICENSE).
