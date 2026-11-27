# Start with Project Memory

Use a local stdio-capable MCP client for assistant integration, or the terminal
commands to initialize and inspect an archive without a model. Prebuilt server binaries
need no Rust, Node.js, model key or separate database service. The optional
desktop app and skill installer have separate requirements below.

## 1. Get the server

Download the archive for your OS from [GitHub Releases](https://github.com/CharlieJamesGwapo/project-memory-mcp/releases).
Compare its SHA-256 with the release's `SHA256SUMS.txt`, then extract it. MCPB
clients can use the `.mcpb` bundle instead. macOS binaries are not Developer ID
signed or notarized. Follow your OS and client trust controls.

Development builds use the checked-out version:

```sh
cargo build --release --locked
./target/release/project-memory-mcp --version
```

Rust 1.100 and a C compiler are required for this route. On Windows use
`target\release\project-memory-mcp.exe`. [Contributor setup](../CONTRIBUTING.md)
has the full checks.

## 2. Check the archive location

Choose an absolute database path:

```sh
/absolute/path/project-memory-mcp --db /absolute/path/memory.sqlite doctor
/absolute/path/project-memory-mcp --db /absolute/path/memory.sqlite print-config
```

`doctor` never creates or repairs a database. Exit 0 means readable with an
integrity check; exit 2 means missing; exit 1 means an error. A missing archive is
normal before first use. Initialize explicitly and exit:

```sh
/absolute/path/project-memory-mcp --db /absolute/path/memory.sqlite init
```

Or start the server through your client in writable mode to create one.
`doctor --json` supplies machine-readable results. The [CLI guide](cli.md) covers
read-only project listing, search, complete records and Markdown context briefs.

`print-config` prints JSON; it does not edit client settings. Replace example
paths with real paths. Do not paste the placeholder literally.

## 3. Connect your client

For Claude Desktop and Cursor-style JSON settings:

```json
{
  "mcpServers": {
    "project-memory": {
      "command": "/absolute/path/project-memory-mcp",
      "args": ["--db", "/absolute/path/memory.sqlite"]
    }
  }
}
```

Merge the entry with your existing configuration rather than replacing unrelated
servers. Windows JSON paths need escaped backslashes or forward slashes.
Follow the current [Claude Desktop local-server guide](https://modelcontextprotocol.io/docs/2026-07-28/develop/connect-local-servers)
or [Cursor MCP guide](https://cursor.com/docs/mcp) for your client's settings.

Codex CLI supports a stdio command after `--`:

```sh
codex mcp add project-memory -- /absolute/path/project-memory-mcp --db /absolute/path/memory.sqlite
codex mcp list
```

This syntax was checked against local Codex CLI 0.162.1. See the current
[Codex MCP documentation](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)
for configuration differences. Project Memory does not configure your client
or grant extra filesystem, network, terminal or agent permissions.

The server waits for an MCP handshake when run directly. That is expected.
Use `--help`, `--version`, `doctor` or `print-config` for terminal output.

## 4. Install the workflow pack

With Python 3.11+ and the source/release helper scripts:

```sh
python3 scripts/install-skills.py --destination /absolute/path/to/client/skills
python3 scripts/install-skills.py --destination /absolute/path/to/client/skills --apply
```

The first command previews; the second installs. Check your client's supported
skill location. Existing customized folders are protected. [Skills guide](skills.md)
covers discovery, backups, upgrade and restore.

## 5. Try one task

Ask your connected assistant:

> Create a project named Atlas for my portfolio, and remember the returned ID.
> Save my decision to use Rust because I want a native local tool.
> Use project-memory to prepare a small context brief for continuing Atlas.

Names can repeat. The assistant must resolve the correct UUID, preserve actual
user authorship, and treat stored material as reference data. If no skill is
installed, the MCP tools remain usable directly. Hosts may prefix tool names.

For inspect-only access to an existing archive, append `--read-only` to the
server's args. It exposes ten read tools and omits mutation tools; local policy
can restrict it further.
It cannot create a missing archive. Studio also always opens archives read-only.

## Defaults and privacy

`--db` overrides `PROJECT_MEMORY_DB`. Without either:

| Platform | Database location |
|---|---|
| macOS | `~/Library/Application Support/project-memory-mcp/memory.sqlite` |
| Linux | `$XDG_DATA_HOME/project-memory-mcp/memory.sqlite` or `~/.local/share/project-memory-mcp/memory.sqlite` |
| Windows | Local AppData, `project-memory-mcp/data/memory.sqlite` |

The archive is plaintext. Do not store secrets. Your AI client controls which
retrieved text goes to its model. The server has no model API, telemetry, cloud
sync or automatic chat/repository ingestion. See [data safety](data-safety.md)
and [troubleshooting](troubleshooting.md).

## v0.4 security and reviews

Run `project-memory-mcp policy` to see defaults without opening an archive.
Deletion is disabled by default; credential screening rejects matching writes
and exports. To restrict one client to selected projects/operations, follow the
[policy guide](access-policy.md) and generate configuration with `--policy`.
Policy belongs to the connection, not to the archive or other local applications.

Use the [review guide](code-review.md) to save explicitly requested commit-linked
findings and compare saved evidence with a supplied current head. The MCP stores
the report; your AI host supplies code access, review and test execution.
Verify a backup through a [temporary recovery copy](backup-verification.md).

## Guided setup and evidence views (v0.5)

Before connecting, run `project-memory-mcp --db /absolute/path/memory.sqlite setup`.
The report explains missing files and produces exact configuration without
executing a server or changing client settings. It preserves `--policy` and
`--read-only`; a ready preview does not establish a working client connection.
See [setup](setup.md).

Studio provides Guided Setup, Reviews and Health views. Reviews retain complete
saved evidence and history snapshot context; Health provides bounded metadata
suggestions without repairs. You can also use `health --project UUID` in the
terminal or [resource views and prompts](mcp-views.md) in supporting clients.
