# Contributing

Read [architecture](docs/architecture.md), [API contracts](docs/api-reference.md)
and [AGENTS.md](AGENTS.md) for the cross-layer invariants. Keep changes focused
and describe the behavior, evidence and relevant limits in a pull request.

Use Rust 1.100.0 and keep Cargo.lock committed. Before proposing a change, run
`cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`,
and `cargo test --locked`.

Add behavior tests for storage or protocol changes. Tests must use temporary
synthetic data, never a developer's real memory database. Keep stdout reserved
for MCP protocol traffic. Preserve explicit project boundaries, immutable
history, bounded outputs, and the distinction between a source reference and
verified evidence.

Update tool documentation and skills when changing their input contracts.
Changes to Cargo version or package identity must also update server.json,
native/npm versions and release instructions. Regenerate locks with the intended
toolchain. `server.mcpb.json` describes an actually published bundle; never invent
its hash or overwrite an existing versioned release asset.

Run helper, skill, documentation and security checks too:

```sh
python3 scripts/check-skills.py
python3 scripts/check-docs.py
python3 scripts/check-source-safety.py
python3 -m unittest discover -s tests -p '*_test.py'
cargo package --locked
```

Verify the generated archive with `scripts/check-package.py`. Its path includes
the current Cargo version; use that version rather than copying an old filename.
Source-policy checks require a Git checkout. Do not commit archives, databases,
tokens, environment files, node_modules or target directories.

For Studio, install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
and Node 26, then:

```sh
cd apps/studio
npm ci --ignore-scripts
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --features tauri/custom-protocol --all-targets -- -D warnings
cargo test --locked --manifest-path src-tauri/Cargo.toml --features tauri/custom-protocol
```

Build the frontend serially before native asset embedding. Do not run another
Vite production build while Tauri compiles its embedded assets. Test actual
native dialogs and data against a temporary synthetic database. Browser sample
success alone does not prove native IPC or another OS works.

Studio term parsing uses committed Unicode ranges generated from the pinned
Rust toolchain, rather than the webview's Unicode database. When upgrading Rust,
regenerate them with `scripts/generate-term-table.rs` using the commands in that
file. The native `term_table` test checks the version and every Unicode scalar;
run it as part of the normal native suite. Keep the generated data and tokenizer
tests together so Node, older webviews and the server agree.

New skill instructions need structural validation and realistic forward checks.
Evaluate unavailable host tools, untrusted memory, stale verification and budget
omissions. Keep model/tool claims tied to actual evidence; avoid fixed savings
or blanket security promises. See [security evidence](docs/security.md).

Open a GitHub issue or pull request after the repository is published. See
SECURITY.md for vulnerability reporting.
