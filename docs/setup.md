# Setup preview

Setup preview checks explicit local inputs and produces JSON stdio configuration
you can review and copy into your MCP client. It does not start the executable,
initialize an archive, open an audit log, or edit a client's settings.

```sh
/absolute/path/project-memory-mcp --db /absolute/path/memory.sqlite setup --json
```

Use `--executable /absolute/path/project-memory-mcp` to preview another installed
server path. The executable check examines metadata only: any existing absolute
regular file can pass. It does not establish executable permissions, binary
authenticity, version, a successful MCP handshake, or client compatibility.

Supply `--policy /absolute/path/policy.json` and `--read-only` when those settings
belong in the client configuration. Paths must be absolute UTF-8 paths. Existing
directories and final-component symlinks are refused. Parent directory aliases,
including normal macOS `/var` aliases, follow the archive's existing path rules.
Arguments remain separate JSON strings, including paths containing spaces.

An example of the configuration shape, using illustrative paths:

```json
{
  "mcpServers": {
    "project-memory": {
      "command": "/absolute/path/project-memory-mcp",
      "args": [
        "--db",
        "/absolute/path/memory.sqlite",
        "--policy",
        "/absolute/path/policy.json",
        "--read-only"
      ]
    }
  }
}
```

The shared Rust entry point is
`setup::preview(database, executable, policy, policy_path, read_only)`. It returns:

| Field | Meaning |
|---|---|
| `status` | `ready` when local checks pass, otherwise `action_required` |
| `checks` | Items with `code`, `status` (`passed` or `action_required`), and a message |
| `configuration` | The complete `mcpServers` object, or `null` when paths or policy cannot be safely reproduced |
| `policy` | The original validated policy summary for the requested server mode, or `null` for invalid policy |
| `diagnostics` | Read-only archive identity, integrity and scoped counts, or `null` when unavailable |
| `next_actions` | Explicit suggested steps; preview does not carry them out |
| `notice` | Inspection and connection-testing limits |

The `connection_not_tested` check is always explicitly labeled. Its `passed`
status means the disclosure is present; it does not record a successful
connection test. `ready` applies to the checks actually performed.

A missing archive produces `archive_missing` and an initialization action. Its
absolute path can still produce valid writable configuration. Preview does not
create its parent directories or the database. A missing executable produces an
installation action and `configuration: null`. Foreign, newer, unreadable, or
damaged archives produce actions and are never adopted, migrated, or repaired.

Archive inspection uses `Store::open_read_only`. This preserves archive records
and schema. SQLite can create or update WAL/SHM coordination files during a
read-only inspection, including while rejecting a foreign WAL archive. Main
archive and existing WAL contents remain unchanged. Preview is not a promise of
an unchanged filesystem tree when inspecting an existing SQLite WAL archive.

The policy summary preserves `audit_enabled` from the supplied configuration.
For inspection, a separate policy clone retains project restrictions and clears
its audit path before application to the read-only Store. No audit log is opened,
created, appended, or tested for writability or header validity. Audit paths that
alias the archive or its SQLite sidecars are refused for copied configuration.
An enabled audit flag does not establish a working audit destination.

Generated configuration preserves the explicit policy path and read-only flag.
A nondefault policy requires a policy file whose loaded content matches the
supplied policy; otherwise configuration is `null`, preventing a copied setup
from silently dropping restrictions. Project-scoped diagnostics count only the
configured project IDs and return no saved record text.

Studio's setup preview takes the paths and options you explicitly enter. It is
separate from Studio's own archive connection and access policy. Copying JSON
does not install it, connect a client, initialize memory, or change Studio's
policy. Browser sample reports are synthetic.
