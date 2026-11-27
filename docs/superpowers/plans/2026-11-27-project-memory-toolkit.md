# Toolkit implementation stages

Design: ../specs/2026-11-27-project-memory-toolkit-design.md
Baseline: Studio9d6bce3; branchfeat/project-memory-toolkit.

1. Research and audit -> fetched primary sources and concrete baseline/security
   reports, existing Studio GitHub CI completed or failures resolved.
2. Design and contract -> written API/UX/skills/security scope and acceptance.
3. Core retrieval -> RED then GREEN tests for unified literal search, bounded
   whole-record bundles, actual serialized byte accounting, atomic explicit
   selection, omissions, unchanged schema and read-only enforcement/discovery.
4. Setup CLI -> doctor/print-config/read-only tests before implementation;
   no missing DB/config mutation or protocol stdout pollution.
5. Skills and installer -> one self-contained router with focused references,
   existing skill upgrades; structural validation, meaningful synthetic forward
   evaluation, installer dry-run/collision/symlink/backup tests.
6. Studio -> integrate task retrieval and byte/token-estimate affordances, race
   tests, production build and actual native verification. Build dist serially.
7. Security/docs -> real Rust/npm audits and source evidence, pin actions/tooling,
   automated checks, README+guide+Mermaid architecture+API+threat model+examples.
8. Integration -> full suites, archive/package checks, independent review,
   versioned artifacts and GitHub updates. New release/Registry metadata only
   after concrete builds and checks; preserve v0.1 published assets.

Root owns CLI/versioning/docs/integration. Bounded agents may own retrieval,
skills/installer and security/CI slices with explicit non-overlapping files.
No simultaneous frontend build while Tauri embeds production assets.
