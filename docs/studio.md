# Project Memory Studio

Studio is the desktop companion to Project Memory MCP. It browses the same local
SQLite archive and helps you assemble a context brief for your next AI session.
The MCP server and workflow pack continue to create notes, decisions and handoffs;
Studio opens existing archives with a read-only SQLite connection.

## Use it

1. Open Project Memory Studio. It looks for the same default database as the MCP
   server. `PROJECT_MEMORY_DB`, when set before launch, overrides that location.
2. Choose **Open database** to select another existing Project Memory database.
   Missing files are never created. An unrelated SQLite database is rejected.
3. Choose a project. Browse notes, search for a literal phrase, inspect decision
   history, or open the complete latest handoff.
4. Open a record and choose **Add to context brief**. Review or remove your
   selection, then **Copy Markdown** or **Save Markdown** using the native dialog.

The brief contains complete selected records with original timestamps,
authorship, source references, decision replacements and every handoff section.
Recorded verification remains historical evidence. Studio does not rerun checks.
The selection is limited to 20 records and 2 MiB of UTF-8 Markdown. Changing
projects, changing archives or refreshing clears the current brief.

**Sample workspace** is an explicitly labeled set of synthetic projects. The
browser development preview uses samples; the desktop app can switch to them
when no archive is available. Samples never represent a connected database.

## Build locally

Requirements: Rust 1.100 (the repository pins it), Node.js 26, and the native
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.
On macOS this includes Xcode Command Line Tools. Linux needs WebKitGTK 4.1 and
the related development packages listed by Tauri.

```sh
cd apps/studio
npm ci --ignore-scripts
npm test
npm run tauri -- dev
```

For a macOS app bundle:

```sh
npm run tauri -- build --bundles app
```

On Windows use `--bundles nsis`; on Linux use `--bundles deb`. The explicit bundle
option overrides the default macOS `app`/`dmg` targets. Those platforms need
their own build hosts; a macOS build is not a Windows or Linux binary.

For a browser-only design preview, run `npm run dev` and open
`http://127.0.0.1:1420`. The browser preview cannot open a live archive.

Run the native service checks from the repository root:

```sh
cd apps/studio && npm run build && cd ../..
cargo test --locked
cargo test --locked --manifest-path apps/studio/src-tauri/Cargo.toml --features tauri/custom-protocol
cargo clippy --locked --manifest-path apps/studio/src-tauri/Cargo.toml --features tauri/custom-protocol --all-targets -- -D warnings
```

## Local data and boundaries

Studio has no accounts, telemetry, model calls or hosted service. Fonts and other
assets ship with the app. Source text is rendered as text; source references are
displayed without opening them. Native IPC exposes a read-only query whitelist,
clipboard copy and a Markdown save dialog. The frontend cannot choose an
arbitrary write path. Exports refuse SQLite files, symlinks and non-Markdown
destinations. Studio does not migrate, adopt or initialize databases.

The macOS Apple Silicon preview is available from GitHub Releases. From v0.2.1
its complete application bundle has an ad-hoc resource seal verified before
packaging and after extracting the exact ZIP. It has no Developer ID signature
or notarization. Studio is a separate download from the MCP server/Registry
package. Existing versioned artifacts remain intact; the security guide records
the earlier v0.2.0 Studio packaging defect and current platform scope.

If you have no archive yet, expand the welcome screen's setup instructions.
With the server installed, `project-memory-mcp init` creates an empty archive and
prints its path without an MCP handshake. Then choose **Open Database**. Use your
connected MCP client to add projects and saved records; Studio remains read-only.

## Toolkit retrieval

The Search tab searches literal text across notes, decision rationale and all
handoff sections. Filter by record kind or include superseded decisions when
reviewing history. Open a result for its full record.

The optional **Suggest context** section offers three presets:

| Preset | Retrieval bytes | Whole records |
|---|---|---|
| Small | 8,000 | 4 |
| Balanced (default) | 16,000 | 8 |
| Expanded | 32,000 | 12 |

Advanced controls allow custom byte/record caps. Presets change settings only; **Build suggestions**
explicitly retrieves records. A task can use the default literal phrase or
opt-in ranked terms; without a task, it uses the recent handoff/active decisions/notes. Matched
terms explain lexical matches, not semantic meaning or confidence. It reports the byte
budget, heuristic token estimate and omissions. Review suggestions before using
**Add suggestions** or **Replace brief**. These are explicit actions; retrieval
does not silently change your manual selection.

Retrieval bytes count the canonical MCP response. Final Markdown bytes and its
heuristic estimate are separate. Neither is a provider billing measurement.
See [the API reference](api-reference.md) for limits and measurement details.
Read-only SQLite can create/update WAL/SHM coordination files in the archive
folder while preventing memory/schema writes. See [troubleshooting](troubleshooting.md).
