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
5. Use **Reviews** for complete supplied review notes, or **Health** for a bounded
   metadata inspection. Each view loads only after your explicit action.

The brief contains complete selected records with original timestamps,
authorship, source references, decision replacements and every handoff section.
Recorded verification remains historical evidence. Studio does not rerun checks.
The selection is limited to 20 records and 2 MiB of UTF-8 Markdown. Changing
projects, changing archives or refreshing clears the current brief.

**Sample workspace** is an explicitly labeled set of synthetic projects. The
browser development preview uses samples; the desktop app can switch to them
when no archive is available. Samples never represent a connected database.

## Guided setup

**Guided setup** is available before opening an archive and above the connected
project workspace. Enter absolute database and server executable paths, an
optional policy path, and the read-only choice for your client's configuration.
Choose **Preview setup** to read diagnostics and inspect the exact stdio JSON.
Choose **Copy configuration** to copy that JSON for an explicit client edit.
Changing a path or the read-only choice clears the previous report and copy
action, including when an earlier preview is still pending.

The native preview inspects executable metadata without running it. It does not
test an MCP connection, initialize an archive, or change another application's
settings. Missing or unsupported files produce next steps. The browser sample
only illustrates the report and JSON; it checks no real filesystem paths.
Studio has its own local read-only session. A previewed policy and the read-only
choice apply to the generated client configuration, not Studio's connection.
See [setup diagnostics](setup.md) for exact checks and limits.

## Reviews and health

The **Reviews** tab calls `get_review_history` with a page limit of 20 and a
16,000-byte canonical response cap. Enter an optional full lowercase hexadecimal
current commit ID (40 or 64 characters), then choose **Load reviews**. Freshness
only compares that supplied value with the recorded head. It does not inspect a
repository or rerun checks. Each entry displays base/head commits and revision
links; opening its full note shows all findings, finding statuses, recorded check
outcomes, authorship, original timestamps and sources. **Add to context brief**
is still explicit, and Markdown preserves the full review metadata.
Opening a review retains the complete loaded history note, including its computed
revision links and supplied-head comparison. The inspector and exported Markdown
label that comparison as evidence from the loaded snapshot; later head edits do
not reinterpret an already selected review as current verification.

The **Health** tab calls `get_memory_health` with the same page limit and byte
cap. Choose **Inspect health** to see snapshot-wide counts, suggestions and a
complete assessment for each record on the page, including records without
signals. “Unrevised reviews” counts the latest review revisions; it does not mean
their heads match the optional supplied commit. Open finding counts include
`open` and `needs_verification` statuses in those latest reviews.

Use **Load more reviews** or **Load more assessments** to continue the bounded
snapshot. Page byte counts apply to each canonical MCP response, not the combined
list. A page with no signals does not establish a healthy entire archive.
Source references are not opened, and multiple active decisions are a prompt
to review their scope, not evidence of a semantic conflict or safety verdict.
See [memory health](memory-health.md) for signal definitions and limits.

Editing the supplied head clears mismatched results and pagination. Changing
project, archive or session discards pending responses. Browsing these views and
previewing setup never adds records to your manual brief. Arrow keys, Home and
End move between project tabs; layouts support narrow windows and reduced motion.
Closing the record inspector with Escape or its Close button restores keyboard
focus to the opener, or a current workspace control if that opener was removed.

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
