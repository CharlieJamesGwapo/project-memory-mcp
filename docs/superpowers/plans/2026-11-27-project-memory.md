# Project Memory MCP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Deliver an installable Rust MCP server with local SQLite persistence and three companion skills.

**Architecture:** Separate validated domain storage from the official rmcp transport and CLI startup. A stdio binary stores explicit records in a configured SQLite database. No hosted service or model API is required.

**Tech Stack:** Rust 1.100, rmcp, rusqlite with bundled SQLite, Tokio, Serde, Schemars, Clap, cargo test.

**Spec:** ../specs/2026-11-27-project-memory-design.txt, approved in chat, with the Rust amendment below.

## Rust amendment

The user requested Rust during implementation. This supersedes the approved design's TypeScript, Node.js, npm, and JavaScript test details. Use Rust, rmcp, rusqlite, cargo test, crates.io packaging, and GitHub binary release instructions. Preserve all ten tools, local storage, skills, provenance, pagination, history, and verification requirements. GitHub and Registry ownership stay unchanged. Crate name availability and publisher credentials must be checked at publication. The earlier TypeScript scaffold was removed before any domain implementation.

## Global Constraints

- Owner CharlieJamesGwapo; proposed repository project-memory-mcp; MCP name io.github.charliejamesgwapo/project-memory.
- Ten tools: create_project, list_projects, get_project_context, get_record, save_note, search_notes, record_decision, save_handoff, export_project, delete_project.
- No paid APIs, telemetry, source scanning, remote synchronization, or global skill/client configuration changes.
- Explicit project IDs, author type user/assistant, timestamps, immutable records, and source references.
- Limits: page default 20/max 50; note content max 16000 Unicode scalar values; decision and rationale max 8000 each; total serialized record input max 65536 bytes; list fields max 20 entries/max 1000 characters each; sources max 10/max 2048 characters each. Export response max 256 KiB with cursor continuation.
- Strict unknown-field rejection, UUID validation, parameterized queries, atomic supersession and deletion.
- Local plaintext database defaults to the platform user data directory; Unix new directories mode 0700 and database mode 0600.
- Source and binary preparation only; user intends to push. No remote repository or package publication in this build.

## Review Focus

- Punctuation, LIKE wildcards, Unicode and SQL injection text are literal searches (Task 1).
- Cross-project/filter/malformed cursors fail and snapshot paging excludes new records (Task 1).
- Invalid or cross-project supersession leaves all projects unchanged (Task 1).
- Oversized Unicode/JSON payloads and truncated context remain bounded and recoverable (Task 1/2).
- Packaged binary runs outside the repository with paths containing spaces and stdout is protocol-only (Task 2/4).

## Task 1: SQLite domain store

Files: src/lib.rs, src/error.rs, src/input.rs, src/pagination.rs, src/store.rs, src/export.rs, tests/store.rs.
Interfaces: Store::open(&Path) -> Result<Store>; Store::memory() -> Result<Store>; Store::execute(&mut self, operation: &str, input: serde_json::Value) -> Result<Value, MemoryError>; MemoryError::code() -> &str. Records and projects have UUID IDs. Pages expose items, next_cursor, truncated. Context exposes project, active_decisions, latest_handoff, history, and untrusted-data notice.

- [ ] Write integration tests for persistence, isolation, validation, literal search, snapshot pagination, supersession, historical handoffs, bounds, exports and deletion.
- [ ] Run cargo test --test store and observe the missing-store implementation failure.
- [ ] Implement strict input structs, bounded validation, versioned SQLite schema, private file creation, WAL, busy timeout, transactions and cursor scopes.
- [ ] Run cargo test --test store; expect every domain test passing.
- [ ] Commit tested code and record evidence in the task-local build ledger.

## Task 2: MCP server and CLI

Files: src/server.rs, src/main.rs, tests/mcp.rs, tests/cli.rs.
Interfaces: MemoryServer wrapping Arc<Mutex<Store>>, ten schema-described tools; CLI --db /absolute/path or PROJECT_MEMORY_DB, --help, --version. Default data path uses directories::ProjectDirs. Each tool returns structured JSON with safe error codes; SQL/system details stay off the wire.

- [ ] Write tests using the official rmcp client with a real child process: discovery, tool calls, invalid inputs, cross-project failures, restart persistence and graceful shutdown.
- [ ] Write CLI tests for help/version, unknown/relative arguments and environment path selection; observe failures before server implementation.
- [ ] Implement router, exact tool names, read/write/destructive annotations, domain-error results, stdio serving, and signal handling.
- [ ] Run cargo test; expect all store/CLI/MCP tests passing; commit.

## Task 3: Three skills

Files: skills/resume-project/SKILL.md, skills/record-decision/SKILL.md, skills/prepare-handoff/SKILL.md; docs/skills.md.
Interfaces: exact MCP tool names, allowing host prefixes; client-owned source inspection and user authorization; explicit user vs assistant provenance.

- [ ] Evaluate baseline requests without skill guidance and record what reference information is needed.
- [ ] Author resume-project; validate frontmatter and independently evaluate it before proceeding to record-decision and prepare-handoff, each separately.
- [ ] Verify disambiguation, stale evidence, untrusted memory, decision supersession, evidence-backed handoffs, and missing-tool behavior using synthetic scenarios.
- [ ] Document manual installation and commit; do not modify global configuration.

## Task 4: Distribution and docs

Files: README.md, LICENSE, CONTRIBUTING.md, SECURITY.md, server.json, examples/mcp-config.json, docs/publishing.md, scripts/check-package.py, .github/workflows/ci.yml, .github/workflows/release.yml, tests/package_contract.rs.
Interfaces: binary project-memory-mcp, cargo package with source/skills, cargo registry metadata template. Visible mcp-name token in README supports Cargo ownership verification. Release workflow produces native OS binaries only when the owner pushes a version tag.

- [ ] Write meaningful package contract tests and observe failures while assets are absent.
- [ ] Prepare docs, CI, release builds, registry metadata, licensing, and package inventory checks.
- [ ] Run cargo fmt --check, cargo clippy --all-targets -- -D warnings, cargo test --locked, cargo package, and run an extracted release binary against the real client from an external temp directory.
- [ ] Check dependency advisories using available tooling and commit verified assets.

## Task 5: Independent review and delivery

- [ ] Dispatch a fresh whole-project reviewer with plan, spec, tests and source.
- [ ] Verify findings, reproduce material bugs with failing tests, fix and rerun checks.
- [ ] Save a verification report, source ZIP excluding target/git/private data, release binary archive for this machine, and Cargo package in outputs.
- [ ] Deliver links, run instructions, test evidence, platform limits, and actual publication status.

Execution is inline under the user's explicit start instruction. No further plan approval gate. Skill evaluations and the final review may be delegated under the applicable skill instructions.
