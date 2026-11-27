# Setup and Context Confidence Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans for root tasks; independent bounded component assignments run in parallel under the existing user GO AHEAD.

**Goal:** Easier local onboarding and trustworthy task retrieval without schema changes or paid services.
**Architecture:** Reuse transactional Store and typed tool inputs, add explicit read-only CLI entry points, an optional ranked lexical bundle mode, and progressively disclosed UI/skill controls.
**Tech Stack:** Rust1.100, SQLite schema1, rmcp, Tauri/React, stdlib Python.
**Spec:** ../specs/2026-11-27-context-confidence-design.md

## Global constraints

Retain stdio default, 12 MCP tools, literal search defaults, full records/sources, UUID isolation, current byte caps and conservative canonical accounting. No network/shell/agent runtime in MCP. Never run Vite build concurrently with Tauri embedding. Existing release assets immutable. No provider-token or malware-absence guarantees.

## Review focus

Concurrent initialization must recheck identity atomically; WAL changes stay outside transaction and after identity validation. Terms must rank before candidate window and use bound SQL. CLI read errors cannot create the main DB. Preset changes cannot mutate the brief or accept stale responses. Model/fixture reports cannot claim current checks or permissions from saved text.

## Tasks

- [ ] Root: tests/store startup regression and tests/cli_workflows.rs RED. Fix Store::initialize in src/store.rs; add focused src/cli.rs/CLI dispatch to main, typed flags, safe stdout Markdown/JSON. GREEN core suite and startup probe.
- [ ] Retrieval worker: own src/input.rs, src/store/retrieval.rs and new term helper below retrieval (avoid src/store.rs edits), tests/retrieval.rs and relevant MCP tests/description. Implement exact spec query_mode and ranking; RED/GREEN core suite; expose ContextQueryMode literal/terms serde enum for CLI consumption. Do not build frontend.
- [ ] UI worker after contract provided: own ContextSuggestions.tsx, sampleRetrieval.ts, types.ts, relevant retrieval/sample tests and scoped CSS. Implement presets/custom/terms with exact spec, error distinctions and sample parity. Tests then npm test/build; tell root before touching dist.
- [ ] Root: add focused debugging/verification skill references and eval fixtures; implement real-MCP evaluation helper and meaningful Python tests. Validate all skills and independently evaluate new routes. Review existing fallback permission gates.
- [ ] Root: update docs and version0.3.0 in all first-party manifests/locks/identity template, keeping actual published server.mcpb.json untouched until real release. Include measured fixture receipts and research rationale, not ranking/cost hype.
- [ ] Integrate all changes, full appropriate tests plus CLI/MCP/native flows. Independent review of core/CLI/ranking and release/skills. Correct verified faults before GitHub integration.
- [ ] Publish verified feature branch/main, four native targets, strict signed Studio ZIP/MCPB, exact new Registry version. Verify public files and publish receipt. Final reply repeats or links LinkedIn v0.2.1 caption and reports completed next upgrade clearly separately.
