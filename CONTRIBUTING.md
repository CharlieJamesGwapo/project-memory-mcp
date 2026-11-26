# Contributing

Use Rust 1.100.0 and keep Cargo.lock committed. Before proposing a change, run
`cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`,
and `cargo test --locked`.

Add behavior tests for storage or protocol changes. Tests must use temporary
synthetic data, never a developer's real memory database. Keep stdout reserved
for MCP protocol traffic. Preserve explicit project boundaries, immutable
history, bounded outputs, and the distinction between a source reference and
verified evidence.

Update tool documentation and skills when changing their input contracts.
Changes to Cargo version or package identity must also update server.json and
release instructions. Package inventory checks must pass before publishing.

Open a GitHub issue or pull request after the repository is published. See
SECURITY.md for vulnerability reporting.
