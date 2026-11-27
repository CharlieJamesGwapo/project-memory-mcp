# Changelog

## 0.2.0 — toolkit upgrade

- Added whole-record context bundles with bounded canonical MCP result bytes,
  explainable omissions, atomic explicit selections and labeled token estimates.
- Added literal search across notes, decisions and handoffs.
- Added read-only MCP mode, archive doctor and printed client configuration.
- Added one project-memory entry skill with token-saving, evidenced review,
  agent coordination and security-review references. Preserved the original
  three focused memory skills and legacy retrieval fallback.
- Added offline skill install/upgrade/restore, skill/document validation and a
  consistent SQLite online-backup helper.
- Added Studio project browsing, full-record inspection, search and deliberate
  context suggestions with explicit apply/add/replace and Markdown output.
- Added architecture, API, setup, workflow, data-safety and security guides.
- Pinned workflow actions, isolated publication permissions, added artifact
  provenance and advisory/source checks, and removed the vulnerable MCPB npm
  build dependency chain through standard-library bundle packing.

Schema remains1. Existing tool inputs are preserved. Runtime and distribution
limits, including upstream desktop warnings, are documented in docs/security.md.

## 0.1.0 — initial release

- Rust stdio MCP with ten tools, SQLite projects, literal note search,
  immutable handoffs and explicit decision supersession.
- Three companion skills and native/MCPB distribution through GitHub and the
  official MCP Registry.
