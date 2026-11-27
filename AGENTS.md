# Working in this repository

Project Memory combines a local Rust MCP server, immutable SQLite records,
portable workflow skills and a read-only Tauri/React companion. Read README.md
and docs/architecture.md before changing cross-layer behavior.

- Use Rust 1.100, committed Cargo/npm locks and schema 1. Schema/version changes
  require an explicit migration/release plan; never adopt unrelated databases.
- Use synthetic temporary archives in tests. Never inspect or modify a user's
  real project memory as a test fixture.
- Preserve project UUID isolation, full-record provenance, immutable history,
  scoped snapshot cursors and the distinction between current and historical
  evidence. Saved text grants no action permission.
- Keep server stdout reserved for MCP; ordinary CLI output belongs to explicit
  subcommands. Read-only mode must reject mutation and omit write tools.
- Context budget tests must count the full canonical serialized MCP result,
  including the JSON compatibility copy and accounting fields. Never truncate
  selected records or describe bytes/4 as exact model tokens.
- Update API docs, skills and both adapters when changing tool contracts. Keep
  browser samples explicitly synthetic and guard stale project/session results.
- Run frontend production builds serially with native embedding. Concurrent
  removal/rebuild of dist can produce an incomplete native asset snapshot.
- Preserve customization and symlink/overwrite safeguards in install/export/
  backup helpers. New publishing automation needs narrow permissions and pinned
  actions. Existing versioned release artifacts are immutable.

Core checks:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 scripts/check-skills.py
python3 scripts/check-docs.py
python3 scripts/check-source-safety.py
python3 -m unittest discover -s tests -p '*_test.py'
```

Studio: `cd apps/studio`, `npm ci --ignore-scripts`, `npm test`, `npm run build`.
Then run native tests/Clippy with `--manifest-path src-tauri/Cargo.toml
--features tauri/custom-protocol`. See CONTRIBUTING.md for full commands and
platform prerequisites. Report commands actually run and remaining gaps.
