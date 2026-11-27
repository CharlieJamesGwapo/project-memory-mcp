# Use Project Memory from a terminal

The v0.3 CLI can initialize an archive and inspect saved memory without an AI
client. It uses the same validated Rust Store as MCP. With no subcommand, the
executable still starts the stdio MCP server and waits for a client handshake.

Replace the executable and database paths below with real absolute paths.
Prebuilt server binaries require no Rust, Node.js, model key or Python.

## Initialize explicitly

```sh
/absolute/path/project-memory-mcp --db /absolute/path/memory.sqlite init
/absolute/path/project-memory-mcp --db /absolute/path/memory.sqlite doctor --json
```

`init` initializes a new or empty archive and exits. Running it again preserves
supported existing projects and records; it adds no example projects or notes.
It refuses foreign/newer archives, a symlink at the database path, and
`--read-only`. Use `init --json` for a ready report and diagnostics. Without
`--db`, it uses the [documented platform default](quickstart.md#defaults-and-privacy).
Use your connected MCP client to create projects and save memory. Studio is an
inspector, not a writer or a client configuration installer.

## Inspect using UUIDs

```sh
project-memory-mcp --db /absolute/path/memory.sqlite projects --limit 20
project-memory-mcp --db /absolute/path/memory.sqlite search --project PROJECT-UUID "authentication"
project-memory-mcp --db /absolute/path/memory.sqlite record --project PROJECT-UUID --record RECORD-UUID
```

These commands return JSON. Names may repeat: copy the exact returned project
UUID rather than choosing a project by name. `search` returns previews; `record`
returns the complete record, including sources and attribution. `search`
supports `--kind note|decision|handoff`, `--include-superseded`, `--limit` and
`--cursor`. `projects` supports `--limit` and `--cursor`. Follow `next_cursor`
with the same command, project, query and filters.

All inspection commands open the archive read-only, even without `--read-only`.
A missing archive is an error, not a reason to create a database. SQLite may
maintain WAL/SHM coordination files; memory/schema contents remain read-only.

## Compose a context brief

```sh
project-memory-mcp --db /absolute/path/memory.sqlite context --project PROJECT-UUID
project-memory-mcp --db /absolute/path/memory.sqlite context --project PROJECT-UUID --query "windows authentication" --query-mode terms --max-bytes 8000 --max-records 4
project-memory-mcp --db /absolute/path/memory.sqlite context --project PROJECT-UUID --record RECORD-UUID --json
```

Default output is a Markdown document containing the complete selected records,
project, provenance, selection/omission metadata and measurement notice. `--json`
returns the structured bundle. The command writes only to stdout; saving its
output to a file is an explicit shell operation under your control.

The default query mode is `literal`. Opt-in `terms` matches distinct query words
across fields, with explainable lexical ranking. It is not semantic search.
See [ranking and limits](api-reference.md#whole-record-context-bundle). Without a
query, the latest handoff, active decisions and recent notes are prioritized.
Repeat `--record` to select up to 20 complete records in a specific order; that
selection must fit completely. Explicit records and `--query` are incompatible.

`--max-bytes` counts the canonical MCP result, including its compatibility text
copy. It does **not** cap the differently formatted CLI JSON or Markdown output.
The bundle's byte accounting is unchanged by CLI rendering. Bytes/4 remains a
heuristic estimate, not a model tokenizer, whole-chat limit or billing receipt.

## Configure and troubleshoot

`print-config` prints a JSON stdio example without editing client settings.
Append `--read-only` when printing an inspection-only configuration. Follow
[client setup](quickstart.md) for Codex, Cursor and Claude Desktop differences.

Successful commands exit 0. Runtime/validation errors exit 1 and print an error to
stderr; argument-parser errors usually exit 2. `doctor` specifically exits 2 for
a missing archive, 0 for ready and 1 for error. Do not assume `--json` makes runtime
errors JSON. Historical saved test results are still historical; inspect current
source and run the relevant checks through your host tools.
