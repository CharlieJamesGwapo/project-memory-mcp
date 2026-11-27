# v0.5.0 guided memory implementation plan

> For agentic workers: use subagent-driven development or executing-plans. Work
> in disjoint files and verify each task before integration.

Goal: deliver the approved usability/evidence upgrade and publish verified assets.
Architecture: shared read-only setup/health APIs, existing immutable review notes,
native Studio bridge and policy-enforced MCP resources/prompts.
Tech stack: Rust 1.100, rmcp 3.12, SQLite schema 1, React/Tauri, stdlib Python.
Spec: ../specs/2026-11-27-guided-memory-design.md

## Global constraints

Preserve schema1, complete provenance and canonical budget accounting. No real
memory fixtures, arbitrary runner, cloud service or automatic repair/capture.
No source changes after build freeze except fixes verified with regressions.

## Review focus

- Setup preview must not create an archive, audit log or client configuration.
- Resource reads must enforce policy even when hidden from template discovery.
- Health pages must preserve snapshots and advance through no-signal rows.
- Studio must reject stale head/project/session responses and preserve manual briefs.
- URI/prompt/health errors must not echo credential values or imply verification.

## Stage 1 — Design and baseline (root)

- [x] Read relevant layers, write contracts, verify clean base and baseline core.
- [x] Start disjoint setup, health and frontend workers with exact interfaces.

## Stage 2 — Setup (worker)

- [x] Add src/setup.rs and tests/setup.rs with meaningful RED/GREEN checks.
- [x] Document report/limits in docs/setup.md. Root wires CLI/native command.

## Stage 3 — Health (worker)

- [x] Add src/health.rs input type and src/store/health.rs; tests/health.rs.
- [x] Test snapshot pagination, full accounting, evidence-only signals and scope.
- [x] Document health contract. Root wires Store/MCP/CLI/security inventory.

## Stage 4 — Studio (worker)

- [x] Add setup/review/health views, native/sample adapters and typed contracts.
- [x] Test real interactions, stale responses and selected-context preservation.
- [x] Freeze frontend before root serial production/native embedding.

## Stage 5 — MCP and release integration (root)

- [x] Add resources/prompts through Store and real stdio tests, enforce restrictions.
- [x] Update skills, all guides/counts, packaging inventories and source versions.
- [x] Full formatting/Clippy/core/native/frontend/Python/package/smoke checks.
- [x] Independent scoped review and fix confirmed material issues with regressions.

## Stage 6 — Publication (root)

- [x] Commit/push, observe core/Studio/security CI and four native targets.
- [x] Merge authorized release, publish new immutable MCPB/Studio/native artifacts.
- [x] Verify checksums/provenance/actual bundle and publish exact tag to Registry.
- [x] Read back metadata, commit public receipt and deliver guides/source links.
