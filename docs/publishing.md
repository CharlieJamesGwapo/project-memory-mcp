# Publish a versioned release

Native GitHub downloads and MCPB can be listed in the official MCP Registry
without crates.io publication or a hosted service. This repository already
exists at CharlieJamesGwapo/project-memory-mcp. Never create a duplicate or
overwrite assets referenced by an existing Registry version.

## Release gate

Use the intended commit with consistent root/native/npm versions, Cargo locks,
`server.json`, user docs, skills and `docs/releases/VERSION.md`. Run core,
frontend, native, helper, archive, advisory and documentation checks. Review the
actual platform/runtime evidence and unresolved upstream warnings. Inspect the
tracked inventory for credentials, databases and generated caches.

`server.json` is the optional Cargo publishing template and identity source.
`server.mcpb.json` describes an actually published MCPB. Generate its download
URL and SHA-256 from the final built artifact; do not invent a future hash.

## GitHub workflow sequence

1. Push the verified source commit. Require CI, Studio and Security checks to
   pass. A source review is separate from a successful compile or audit.
2. Dispatch **Prepare release binaries** from that commit/ref. It tests/builds
   Linux x64, macOS arm64, macOS Intel and Windows x64, then archives binaries,
   docs, all skills and user helper scripts. Require all four jobs to pass.
3. Dispatch **Publish native release** from the same intended release commit,
   passing the successful preparation run ID. It verifies the producing workflow,
   repository, ancestry and unchanged Rust source/locks before using artifacts.
   The read-only preparation job assembles a universal macOS server, stages and
   validates the MCPB, unpacks/smoke-tests the actual result, and generates exact
   Registry metadata and checksums. It also builds the macOS arm64 Studio preview,
   verifies its complete ad-hoc bundle signature, and repeats verification after
   extracting the exact ZIP that will be published.
4. A separate job attests exact release assets using GitHub build provenance.
   The final publishing job verifies the MCPB attestation and creates a new
   versioned GitHub Release with narrow repository-write permission. It never
   replaces an existing release.
5. Download the published files, compare their hashes and inspect metadata.
   Verify the artifact attestation against the repository, signer workflow and
   exact release source digest. Check the public bundle URL before listing it.
6. Dispatch **Publish MCP Registry listing** at the exact same release commit or
   its tag. The preparation job verifies matching metadata, bundle hash and
   attestation, and validates a Sigstore-verified pinned publisher. The final
   job uses GitHub OIDC identity to publish without a stored personal token.

The Registry workflow intentionally requires new-release provenance. It is not
an instruction to republish the older, unattested v0.1 assets. Namespace case
matters: `io.github.CharlieJamesGwapo/project-memory`.

The desktop app is optional and distinct from the Registry server package.
The initial Studio download covers macOS arm64 with an ad-hoc signature, no
Developer ID identity or notarization. Other desktop platforms need separate
runtime/distribution review. See [security evidence](security.md).

## Inspect MCPB locally

MCPB is a ZIP containing root `manifest.json` and the server assets. This project
uses Python 3.11+ standard-library packing, eliminating the former npm MCPB build
dependency chain. The validator checks this repository's generated contract;
it is not a general validator for third-party bundles.

Use a clean checkout of the matching release tag when validating a downloaded
bundle. The validator compares supporting asset bytes with that checkout;
modified docs/skills or a newer `main` can correctly fail an older bundle's
validation. For v0.2.1 use the `v0.2.1` source, not a modified working tree.

After obtaining/checking real platform binaries and making a universal macOS
server with `lipo -create`:

```sh
python3 scripts/package-mcpb.py --macos-universal /path/mac-universal --linux-x64 /path/linux-x64 --windows-x64 /path/windows-x64.exe --output-dir /path/staging
python3 scripts/package-mcpb.py --validate /path/staging
python3 scripts/package-mcpb.py --pack /path/staging --archive /path/project-memory.mcpb
python3 scripts/package-mcpb.py --unpack /path/project-memory.mcpb --output-dir /path/unpacked
python3 scripts/smoke-test.py /path/unpacked/server/darwin/project-memory-mcp
```

Use the executable for your current OS for the smoke check. The script uses a
temporary synthetic archive and verifies actual MCP discovery, calls, isolation,
rejections, persistence and shutdown. Never substitute placeholder binaries in
a release or treat synthetic archive tests as a real binary smoke result.

## Optional crates.io route

Cargo publication is separate. Verify the source archive with
`scripts/check-package.py`, use the configured account's normal authentication,
and publish the exact tested version. Do not commit tokens. Only then use a
`registryType:cargo` listing that points at a real available crate version.
The native MCPB route works independently of this option.

## What provenance establishes

Checksums identify the downloaded bytes. GitHub attestation links them to the
repository workflow/source digest. Sigstore verifies the publisher download's
identity. None establishes malware absence or substitutes for source/dependency
review, OS signing or the client's authorization rules.
