# Project Memory Studio Design

The user authorized building the recommended Tauri + React desktop companion with project browsing, memory search, a decision timeline, a handoff viewer, and selected-record Markdown context export. Their “GO AHEAD” directs implementation; proceed with stated engineering defaults without another design questionnaire. Detailed choices below are engineering decisions.

## Intent

Make saved Project Memory records understandable and useful through a local desktop interface. Users should see the reason behind a decision, review recorded progress, and carry a deliberately selected context brief into another AI session.

## Scope and approach

Use Tauri with React/Vite and the existing Rust SQLite library. A separate local web server would add a background process for desktop users; a remotely hosted dashboard would require data transfer, accounts and hosting. The native desktop approach reuses Rust, provides native database/save dialogs and clipboard, and can operate offline.

Build inside `apps/studio`, keeping the released MCP CLI and ten-tool protocol intact. The root library gains read-only opening and two Studio-only read operations. Studio's Rust IPC bridge whitelists read operations and opens existing databases with SQLite read-only/query-only flags. It must never create a missing database, adopt another application's file, chmod it, or invoke write tools. Existing data schema remains version 1.

The app first tries the same configured/default database path as the CLI. If none exists, show a useful empty state with Open Database and an explicitly labeled sample-workspace option. A browser preview uses only labeled synthetic sample data. Samples never masquerade as a real connected database.

## Product behavior

- Project browser with name, purpose, exact note/decision/handoff counts, active decision count, and last activity. Duplicate names remain distinct UUID projects.
- Overview with current decisions, recent notes and the latest recorded handoff/next actions.
- Notes search with literal substring matching and paginated server results. Empty/loading/error states are explicit.
- Decisions timeline includes active and superseded decisions, reasons, authorship and timestamps. Full records are retrieved when opened.
- Handoff view shows completed/unfinished work, next actions, blockers, recorded verification and sources. Recorded verification is not presented as a newly run check.
- Context brief supports selecting up to 20 full records from the current project, viewing/removing selections, copying Markdown and saving through a native save dialog. It must not silently export preview text or a partial selection after an error. Limit generated text to 2 MiB.
- Refresh updates data written by the existing MCP server. Database/project changes clear selections and invalidate old asynchronous UI results. Every native database session has an ID; the backend rejects requests for an old session after switching files.
- Existing memories are displayed as text, never executed or injected as HTML. Source references are displayed, not fetched automatically.

## Interface direction

An editorial archive workspace: warm linen surfaces, deep forest/ink typography, restrained burnt-orange accents, generous spacing, serif project headings and clear sans-serif UI text. Self-host fonts/assets. Use a project sidebar, project header and tabs, readable record detail, and a context-brief panel. Avoid decorative controls without behavior. Support keyboard navigation, visible focus, useful accessible labels, reduced motion and widths down to a small desktop window. User-facing language concerns projects and records rather than IPC, SQLite internals or implementation errors.

## Boundaries and defaults

This release reads, selects and exports; memory edits remain in the existing MCP workflows. No accounts, telemetry, hosted backend, AI API, cross-device synchronization, automatic source indexing, or automatic skill/client installation. The application is a separate desktop product version 0.1.0; do not republish the existing MCP Registry version or mutate released v0.1.0 server assets.

Native file reads use the OS picker or the known CLI database location. Markdown saves use a native user-selected `.md` destination; no arbitrary destination path is accepted from the frontend. Clipboard writes happen only through a user action. The app restricts network/content policy to bundled UI and local IPC. Publication of a new Studio release is separate from building a reviewable application.

## Validation

Core regression tests must prove read-only queries, absent/foreign database refusal without writes, live visibility of MCP writes, exact project counts, filtered pagination and snapshot consistency. Desktop service tests cover whitelists, session isolation, failed-open preservation, missing data and context-size limits. Frontend tests cover full-record formatting, selecting/removing/copying context, empty/error states and project switching. Build/type-check the frontend; visually inspect the interface through the supported browser UI, and test the actual packaged native app against a synthetic MCP database. Retain the existing MCP tests and package checks. Perform an independent final review and fix verified material findings before delivery.
