# Assess a release

Tie the assessment to an exact source commit and proposed version. Identify
the artifacts and supported platforms the release actually promises. Use the
project's maintained release procedure and current check results, not a generic
count of passing tests or an old handoff.

Verify version agreement, guides and compatibility notes, relevant regression
checks, package inventories and exclusion of private data. For produced assets,
check downloaded bytes against checksums and available provenance tied to the
expected source/workflow. For an MCP release, validate the actual bundle's tool
contract and installed behavior rather than testing only a development binary.

Record build/test CI separately from interactive platform coverage. State known
advisories, signing/notarization status and other material distribution limits.
A signature or clean advisory scan is scoped evidence, not a malware guarantee.

Return ready/not-ready for the stated scope, supporting receipts and unresolved
gates. An assessment does not itself publish. If publication is authorized,
preserve previous versioned assets, verify the published release and Registry
readback, and report a confirmed outcome rather than an initiated workflow.
Do not retry a non-idempotent publication blindly after a timeout; inspect the
remote state first.
