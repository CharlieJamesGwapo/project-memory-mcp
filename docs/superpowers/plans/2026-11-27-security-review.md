# v0.4.0 security and review implementation plan

> For agentic workers: use subagent-driven development or executing-plans, with
> disjoint ownership and test-first changes.

Goal: ship the approved security, review and skill upgrade and publish verified artifacts.
Architecture: Store-enforced local policy and credential guards; schema-1 review notes;
progressively loaded skills and standalone safe backup verification.
Tech stack: Rust 1.100, SQLite, rmcp, React/Tauri and standard-library Python.
Spec: ../specs/2026-11-27-security-review-design.md

## Global constraints

Preserve schema 1, UUID isolation, complete records and canonical budget accounting.
Use only synthetic temporary archives; no paid services or real user data as fixtures.
No automatic shell/network/model runtime. Keep prior versioned artifacts immutable.

## Review focus

- Restricted project lists must paginate correctly even with interleaved hidden rows.
- Invalid policy must fail before archive creation; print-config must retain restrictions.
- Sanitized credential reports must not echo values or skip nested review data.
- Review revisions cannot cross projects or fork an existing revision.
- Audit failure must not produce an unreported write or claim cross-file atomicity.

## Stage 1 — Security boundary (root)

- [x] Read current Store/server/CLI and tests; run baseline core tests.
- [x] Add failing policy/credential tests in tests/security_policy.rs.
- [x] Implement src/security.rs, Store policy enforcement and filtered pagination.
- [x] Add sanitized credential inspection, optional audit receipts and CLI integration.
- [x] Verify denial/error paths with core tests and real MCP subprocesses.

## Stage 2 — Review records (independent worker)

- [x] Add src/review.rs types and src/store/reviews.rs methods, tests/reviews.rs.
- [x] Test immutable note representation, chain validation, limits and stale reporting.
- [x] Root integrates dispatch/server routes and policy operation inventory.
- [x] Worker documents contract in docs/code-review.md.

## Stage 3 — Skills and recovery (independent workers)

- [x] Expand focused skill references, evaluations and workflow guide; validate all four folders.
- [x] Add backup verification helper and meaningful negative/restore tests.
- [x] Keep packaging guide/helper inventories explicit and test their presence.

## Stage 4 — Integration and review (root)

- [x] Update versions to 0.4.0; update API/setup/security/architecture/release docs.
- [x] Run Rust formatting, Clippy, all core/native/frontend/Python checks and policy validators.
- [x] Build frontend/native serially, verify actual MCP workflow and retrieval evaluation.
- [x] Independent scoped review; reproduce and fix material findings with regressions.

## Stage 5 — Publication (root)

- [ ] Commit/push feature branch; observe three-OS CI/security/four native target success.
- [ ] Merge authorized changes, publish new native/MCPB/Studio artifacts and attestations.
- [ ] Publish exact tagged bundle metadata to official Registry.
- [ ] Download, checksum, provenance, actual bundle smoke/evaluation and Registry readback.
- [ ] Save public verification receipt and final user-facing guides with known limits.
