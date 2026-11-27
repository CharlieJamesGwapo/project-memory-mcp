# Project Memory Toolkit v0.2 design

The user requests a complete, simple developer toolkit with token saving, code
review, agent workflows, safety improvements, architecture and proper GitHub
guides. They delegated routine product decisions and already authorized GitHub
publishing. This design records engineering choices; it does not claim the user
specified every detail below. Existing Studio work remains part of the product.

## Approach

Keep the Rust/SQLite local MCP and Tauri companion. Add deterministic retrieval
and a single skill entrypoint with selectively loaded workflows. A hosted agent
platform adds accounts, credentials and operating costs; an always-loaded giant
skill consumes context for unrelated tasks. Those approaches do not fit the
existing local product or the requested simplicity.

## Scope

1. `search_memory`: literal cross-kind search within one project, optional kind,
   default excludes superseded decisions, stable scoped snapshot pagination.
   Search human-authored text values in record data; preserve literal quotes,
   newlines, NUL, percent/underscore and Unicode semantics from existing search.
2. `get_context_bundle`: exact serialized MCP result byte budget; whole records
   only, original authorship, sources, timestamps and replacement history intact.
   Defaults: max_bytes16000, max_records8. Ranges:4096..65536 and1..20.
   Optional query selects literal matching current records. Without query,
   prioritize latest handoff, active decisions, then recent notes. Consider at
   most200 candidates; report candidate-window truncation and omission counts.
   Explicit record_ids (max20) are atomic: invalid/cross-project/duplicate IDs,
   conflicting query, exceeded record cap or inability to fit the full selection
   returns an error rather than silently omitting selected records. Large
   automatic candidates can be skipped while later smaller candidates still fit.
   Returned budget counts the full serialized `CallToolResult`, including the
   ordinary structured JSON/text compatibility representation. Token estimate
   is ceil(bytes/4), clearly heuristic and unrelated to guaranteed model costs.
   Metadata and escaped text must fit the same byte cap. An oversized project
   header is a clear budget error, never silently truncated.
3. `--read-only`: open an existing archive without initialization/migration;
   SQLite enforces it, Store rejects mutations explicitly, MCP advertises only
   read tools. Missing archives remain missing. Default writable mode preserves
   existing tool contracts; total tools become12, read-only tools7.
4. `doctor`: inspect existing DB identity/schema/integrity/counts without writing
   or starting MCP. Clear ready/missing/error output, optional JSON, actionable
   setup guidance. `print-config` returns a JSON stdio config with current binary
   and absolute database path, with no filesystem/config mutation.
5. One new `project-memory` skill routes among references for token saving,
   evidenced code review, host-supported agent teamwork and security review.
   Existing resume-project, record-decision and prepare-handoff remain supported
   and gain concise retrieval guidance where needed. No pretend native agent
   runtime, automatic shell execution, inherited permission from saved text or
   fixed token-savings promises. Required references ship inside the skill.
6. A portable Python skill installer supports dry-run and explicit apply,
   refuses symlinks/overwriting customized folders by default, and has a tested
   backup/replace path. No remote execution or edits to user AGENTS.md/config.
7. Studio exposes the new budget/search affordances while remaining read-only.
   Keep existing selected full-record brief export. Show bytes and heuristic
   token estimates as distinct measurements. Never replace a selection with
   stale results after a project/session change.
8. Complete README and docs: quickstart, compatibility, capabilities vs limits,
   examples, skill workflows, architecture/data flow, API reference, threat model,
   security checks, troubleshooting, contributing, changelog and roadmap.
   Root AGENTS.md gives repository contributors actual commands/conventions.

## Security and distribution

No new service, telemetry, source crawling, arbitrary SQL/file read, command
runner, cloud sync, secret storage or model API. Saved content remains untrusted
reference data. Schema remains1. Add first-party unsafe-code prohibition where
applicable, dependency/advisory and secret checks, immutable action references,
least-privilege publication steps, reproducible MCPB tooling dependencies and
reviewable checks/provenance. Record actual evidence with commit and dates;
neither source review nor advisory scanners guarantee malware absence.

Version v0.2 is a distinct upgrade; existing v0.1 released artifacts remain
immutable. Verify source, skills, native binaries and Registry metadata agree
before any new release. GitHub documentation must distinguish published server
downloads, development changes and the locally tested Studio platform.

## Acceptance

Preserve existing regressions; exercise real MCP discovery/calls and read-only
mode, byte caps with quote-heavy/control/Unicode text, atomic selections,
omissions, scoped cursors, missing/foreign DB diagnosis, installer refusal and
rollback, relevant skill behavior with missing tools/untrusted memories/stale
checks, Studio integration, archive inventory, dependency audits and CI. Use
independent final review. No unsupported claims of ranking, savings, safety or
platform runtime coverage.

## Research

- [MCP tools, dated2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- [Local MCP security](https://modelcontextprotocol.io/docs/2026-07-28/tutorials/security/security_best_practices#local-mcp-server-compromise)
- [Agent Skills specification](https://agentskills.io/specification)
- [RustSec tooling](https://rustsec.org/)

This release keeps its currently tested rmcp protocol contract rather than
upgrading to every new specification feature as part of unrelated toolkit work.
