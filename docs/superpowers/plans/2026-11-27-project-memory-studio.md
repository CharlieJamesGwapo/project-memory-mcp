# Project Memory Studio Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans for continuous execution, test-driven development, and one independent final review. A bounded frontend slice may run in parallel under fable-mode's delegation rule using the shared bridge contract.

**Goal:** Deliver a working Tauri/React desktop companion that reads Project Memory and builds exportable context briefs.

**Architecture:** The root Rust library supplies read-only SQLite access and bounded Studio queries. A Tauri bridge manages read-only sessions, native dialogs, clipboard and exports. A React/Vite UI shares an explicit typed contract with a sample adapter for browser QA.

**Tech Stack:** Existing Rust 1.100/SQLite, Tauri 2, React, TypeScript/Vite, local fonts, Rust integration tests and frontend component/unit tests.

**Spec:** ../specs/2026-11-27-project-memory-studio-design.md
**Contract:** ../../../apps/studio/CONTRACT.md (repository root `apps/studio/CONTRACT.md`).

## Global constraints

- User's GO AHEAD authorizes implementation of the recommended application; no further design approval gate.
- Existing MCP remains ten tools, schema version 1; no republishing or changing immutable released artifacts.
- Existing files only, SQLite read-only/query-only, no chmod, no missing-file creation, no write-tool IPC.
- Session IDs reject old requests after file changes. Selection resets on project/session changes.
- Records must be retrieved in full before selected context export. Maximum 20 selected records / 2 MiB Markdown.
- Native user-selected Markdown destination; no arbitrary frontend file-write path.
- Real empty/error/loading/canceled states; clearly labeled samples; no fake operational buttons or external interface assets.

## Review focus

- Default/missing/foreign/read-only/unreadable database paths and metadata must never mutate files (Task 1/2).
- Filters and pagination must not hide later matching records or cross projects (Task 1/3).
- Project/database changes during searches or record fetches must not show another session's data or export stale selections (Task 2/3).
- Long Unicode, Markdown fences, malicious HTML and recorded verification must preserve data as text and never imply fresh verification (Task 3).
- Native build and browser samples are different environments; verify actual native commands and packaged app with a synthetic database (Task 4).

## Task 1: Read-only store and Studio queries

Files: src/store.rs, src/store/studio.rs, src/input.rs, src/paths.rs, src/main.rs, tests/studio_store.rs.
Interfaces: Store::open_read_only(&Path); shared default_database_path(); Store::execute operations studio_list_projects/records as specified in contract. Keep original operations intact.

- [x] Write tests for no missing-file creation, foreign-file/permissions preservation, mutation rejection, concurrent MCP-write visibility, exact counts, kinds, scoped cursors and supersession snapshots.
- [x] Observe missing-API/operation failures before implementation.
- [x] Implement read-only opening, shared paths, strict input types, parameterized summary/filter queries and cursor reuse.
- [x] Run root cargo test and Clippy; commit tested library changes.

## Task 2: Native session service and Tauri app

Files: apps/studio/src-tauri/Cargo.toml, build.rs, tauri.conf.json, capabilities/default.json, src/{lib,main,service}.rs, tests/service.rs.
Interfaces: bootstrap/choose_database/memory_query/copy_context/export_context exactly as shared contract. Session service testable without an interactive window; native command wrappers use dialog callbacks and blocking workers where appropriate.

- [x] Write service tests for read whitelist, stale sessions, failed-open preservation, missing DB and Markdown bounds; observe failure.
- [x] Implement state, same default DB path, native picker, clipboard and `.md` save with destination selected in Rust. Add local-only CSP and scoped window settings.
- [x] Build and run service tests; run native check/Clippy; commit.

## Task 3: React interface and context brief

Frontend worker owns apps/studio outside src-tauri. Files: package/lockfiles, Vite/TypeScript configs, src UI/adapters/types/styles/test files and local assets.
Interfaces: contract exactly; no edits to Rust. Warm editorial workspace, native-aware data adapter, clearly labeled samples, project browser/overview/notes/timeline/handoffs/full-record detail/context panel.

- [x] Write tests for full-record context formatting, selection/removal, project switching, source/error/cancel behavior and sample adapter reads; observe failures before logic.
- [x] Implement accessible, responsive UI with actual interactions and no remote assets.
- [x] Run npm test, build/typecheck and dependency audit; report changes and exact test evidence.
- [x] Root integrates bridge and native menu behavior; commit frontend.

## Task 4: End-to-end verification and distribution

- [x] Run frontend build/tests, root Rust regression tests, desktop service tests and native build.
- [x] Inspect browser UI through supported cua_repl; exercise all tabs, search, details, selection, copy/save and narrow viewport. Fix observed issues.
- [x] Create a synthetic database using the real store, launch the packaged native app, open it and verify real data/clipboard/export. Check read-only database bytes/schema remain intact.
- [x] Run an independent whole-change review; reproduce material findings and fix with regression checks.
- [x] Provide native macOS app/archive, source ZIP, screenshot, setup documentation and verification report; retain other-platform limitations explicitly.
- [x] Keep work on the Studio feature branch; do not republish existing MCP assets or create a new Registry version as part of local implementation.

## Final validation notes

- Core31, native6, frontend23 and packaging Python3 tests passed (63 total). Root and native Clippy, formatting, TypeScript/Vite and npm audit passed.
- Native macOS arm64 production app exercised against a synthetic real SQLite file: missing startup, picker, empty project, full notes/handoffs, literal search, 20→26 pagination, clipboard, native Markdown save, cancellation and live external-write refresh. Browsing/export preserved DB bytes and schema1.
- Independent review found three frontend recovery bugs; all reproduced and fixed with permanent regressions, then independently rechecked.
- First packaged asset failure corrected with a serial frontend/native build, a production embedded HTML/JS/CSS test and build guard. Cargo archive patterns anchored at repository root after the package checker detected nested dependency licenses. Corrected archive39 files passed verification.
- macOS executable has an ad-hoc signature, with no Developer ID signature/notarization. Other-platform CI is configured; Windows/Linux runtime verification is not claimed.
