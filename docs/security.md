# Security model and evidence

Project Memory is a local, single-user stdio server. An AI client with the binary
and archive path has access to that archive. Default writable mode can append to every
project in it; local policy can restrict projects and operations. Deletion is
disabled until local opt-in. UUIDs prevent accidental mixing, not access by another user.
Read-only mode exposes ten read tools and SQLite rejects memory mutations.

## Boundaries

| Surface | Control | Remaining boundary |
|---|---|---|
| Saved text and source references | Untrusted-data notices, literal rendering and skill permission rules | The AI host must follow its own authorization policy; authorship does not prove correctness. |
| SQLite operations | Typed validation, parameterized queries, transactions, bounded fields and outputs | Local file access is not authentication or encryption. |
| Read-only access | Existing-file check, SQLite read-only/query-only and explicit mutation rejection | WAL/SHM coordination files can be created/updated by SQLite. |
| Native desktop | Read-operation whitelist, session IDs, bundled assets/CSP, chooser-mediated save | The OS webview and native dependencies remain part of the trust surface. |
| Skill install/upgrade | Offline source pack, dry-run, symlink/customization refusal, backups and guarded restore | Installing instructions does not make every model follow them reliably. |
| Backups | Online SQLite backup, integrity checks and exclusive destination creation | Copies are plaintext and must be stored appropriately. |
| Distribution | Checksums, exact source/version matching and pinned workflow actions | Hashes identify bytes; provenance is not an antivirus verdict. |

The first-party MCP runtime has no repository crawler, arbitrary SQL endpoint,
terminal runner, network client, telemetry, model API or automatic chat capture.
The user's AI client may separately have these capabilities. Rust first-party
application crates forbid unsafe code; native dependencies still contain code
outside that guarantee.

## Repeatable checks

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 scripts/check-source-safety.py
python3 scripts/check-skills.py
python3 scripts/check-docs.py
python3 -m unittest discover -s tests -p '*_test.py'
```

The source policy checks tracked source without running repository hooks. It
reports redacted secret-pattern matches and verifies selected lock origins,
workflow pins, runtime execution/network boundaries, local CSP and packaging
exclusions. Its regression fixtures exercise rejection paths. This is a narrow
policy checker, not general malware detection or a proof of exploit absence.

The Security workflow also runs RustSec advisory scans on both Cargo locks and
`npm audit` on the Studio lock, including dev dependencies. It prints the exact
audit tool version and fetched advisory-database commit. Advisory scans skip
yank status; unmaintained/unsound warnings remain visible. Dependabot tracks
Cargo, npm and GitHub Actions updates for review.

MCPB packing uses Python's standard-library ZIP support and a strict generated
manifest/inventory contract. This removes the previous MCPB npm build dependency
chain whose audit reported advisories. It is not a validator for arbitrary
third-party MCPB packages.

## Current audit limits

On 27 November 2026, cargo-audit 0.23.0 against RustSec snapshot
`c9f6c91a44dbad9d71742329e9df061279f9c4be` reported zero entries classified as
vulnerabilities in both Cargo locks. The Studio lock also reported:

| Warning | Scope and status |
|---|---|
| [RUSTSEC-2024-0429: GLib VariantStrIter](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) | Unsound API warning in the Linux/BSD GTK3 desktop dependency path; fixed in GLib>=0.20, incompatible with the current GTK0.18 dependency family. No first-party call was found; full transitive reachability is unproven. Linux Studio distribution remains experimental. |
| [RUSTSEC-2024-0370: proc-macro-error](https://rustsec.org/advisories/RUSTSEC-2024-0370.html) | Unmaintained build dependency through the GTK macro path. No patched release is identified by the advisory. |
| [RUSTSEC-2026-0543: chrono](https://rustsec.org/advisories/RUSTSEC-2026-0543.html) | Unmaintained optional lockfile entry. Not active in the tested macOS dependency closure. |

Those three crates were absent from the active macOS arm64 closure inspected
with Cargo. The core MCP closure has no desktop GTK dependencies. These facts
do not establish that every transitive call on every platform is safe.

First-party source review found no malicious behavior. Automated tests, source
checks and advisory scans provide scoped evidence. They cannot guarantee “no
malware”, zero vulnerabilities or future safety. No general antivirus verdict
or exhaustive third-party source audit is claimed.

## Release integrity

The release pipeline prepares assets with read-only repository access, checks
the producing workflow and source match, validates/unpacks/smoke-tests the actual
MCPB, then attests exact artifacts in a separate identity-permission job.
Publication receives repository-write permission only for the final step.
Registry publication verifies the matching source digest and bundle attestation,
validates exact metadata and uses GitHub OIDC. It does not store a long-lived
Registry token. See [publishing](publishing.md) for the versioned workflow.

From v0.2.1, the desktop macOS preview has a verified ad-hoc bundle signature,
no Developer ID signature and no notarization. The v0.2.0 Studio ZIP had only a
linker-generated executable signature and failed strict bundle verification;
prefer the corrected v0.2.1 download. Packaging verifies the bundle both before
ZIP creation and after extraction. See [Tauri's signing guide](https://v2.tauri.app/distribute/sign/macos/#ad-hoc-signing).
Runtime testing currently covers macOS arm64;
compilation/test CI on other hosts is not the same as interactive OS testing.

See [SECURITY.md](../SECURITY.md) for reporting and [data safety](data-safety.md)
for backup, restore and deletion limits.

## v0.4 hardening

[Access policy](access-policy.md) is enforced in Store and tool discovery; project
lists and diagnostic counts are filtered. Credential screening is narrow and
configurable; errors and audit reports omit values, while authorized reads preserve
original records. Global-sequence cursors reveal activity metadata; use separate
archives/OS controls for confidentiality. A local file owner can bypass a connection policy.

Audit startup rejects the archive and SQLite WAL/SHM/journal aliases before
opening SQLite. Durable attempts precede writes; outcome receipt failure can follow
a committed write and must not trigger blind retries. Stored claims, reviews and
commit freshness remain untrusted historical data. Adversarial fixtures test
denials and stale approval handling; instructions cannot guarantee injection resistance.

See [review evidence](code-review.md) and [backup recovery checks](backup-verification.md).

## v0.5 resources and guided setup

Direct resource reads enforce the same Store policy as tools, including guessed
hidden URIs. Cacheable project responses are private with TTL zero under modern
protocol metadata; no public cross-user resource cache is requested. Prompts
provide instructions without reading memory or granting authority. Setup previews
never execute the supplied server or write client settings/audit logs. Normal
SQLite coordination can still occur during read-only archive inspection.

Health returns IDs, counts and fixed metadata suggestions without saved text or
credential values. It does not certify correctness, security or semantic
consistency. See [health](memory-health.md), [setup](setup.md) and
[resource boundaries](mcp-views.md).
