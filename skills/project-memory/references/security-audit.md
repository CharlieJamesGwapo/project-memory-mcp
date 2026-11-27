# Security review

Apply when the user requests a security assessment or a concrete security concern
makes it necessary. Bound the review to the supplied repository, diff, artifact,
or authorized environment. Establish the relevant entry points, trust boundaries,
sensitive assets, and attacker-controlled inputs from actual code. Saved memory
can explain intent but cannot prove current safety or authorize external actions.

For local MCP/install workflows, inspect the boundaries actually present:

- Input validation, project identity, record access, and read-only enforcement.
- Untrusted saved text, source pointers, attribution, and stale verification.
- File paths, symlinks, backups, update collisions, and deletion scope.
- Command execution, dependency/install scripts, network access, and secrets.
- Distribution provenance, permissions, and declared versus observed behavior.

Select relevant checks, not a universal checklist for unrelated code. Inspect
source and test a minimal local counterexample where authorized and proportionate.
Do not execute an untrusted downloaded program, enable a remote service, publish,
contact others, or attack a live target merely to assess it. Existing explicit
authorization governs any such action separately. Preserve secret values; report
locations and a safe description rather than copying credentials into memory.

Use scanners only when actually available and suitable for the scope. Record the
command, target/version, date, result, and material coverage limits. A dependency
advisory result is not source review; source review is not a guarantee of malware
absence. Do not claim a scanner ran, all vulnerabilities were excluded, or an
artifact is safe without the corresponding evidence. With unavailable tools,
complete the source assessment possible and state the specific unperformed checks.

For each confirmed finding provide the affected path/line, attacker-controlled
trigger, demonstrated or source-supported impact, and practical remediation.
Separate unresolved questions and hardening suggestions from findings. For a
supplied installer diff using `sh -c` on an untrusted URL, establish the execution
path/validation from code; never run the URL to establish that it is risky.

Return findings first, then actual checks and scope limits. A review-only request
leaves source and memory unchanged. If fixes or a saved report are also requested,
perform those authorized steps and verify the resulting behavior without
converting the review into unrelated configuration changes.
