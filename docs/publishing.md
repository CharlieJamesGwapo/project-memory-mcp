# Publish Project Memory MCP

The source, executable, and listing metadata are prepared locally. They are not
automatically published by building or testing. Use your own authenticated
accounts; do not paste registry tokens into chats or commit them.

## 1. Publish the GitHub repository

The intended owner is CharlieJamesGwapo. First check whether
`CharlieJamesGwapo/project-memory-mcp` already exists and whether it is the
repository you intend to update. These commands are for a new repository only:

```sh
gh auth status
gh repo create CharlieJamesGwapo/project-memory-mcp --public --source=. --remote=origin
git push -u origin HEAD:main
gh repo edit CharlieJamesGwapo/project-memory-mcp --default-branch main
```

The GitHub CLI may ask for authentication or confirm account permissions.
Review `git status` and the tracked files before pushing. The source ZIP omits
Git metadata; if starting from that ZIP, first initialize a repository, add its
source files, and make an initial commit. Never add your memory database.

## 2. Verify the release

Run formatting, Clippy and tests, then package:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo package --locked
python3 scripts/check-package.py target/package/project-memory-mcp-0.1.0.crate
```

The helper requires Python 3.11 or newer and only reads the archive. Review the
crate inventory and licenses. Test the actual release binary with an MCP client
outside the source directory and use a temporary synthetic database.

## 3. Publish a Cargo package

Check that the crate name `project-memory-mcp` is available or owned by you on
[crates.io](https://crates.io/). GitHub authentication does not establish Cargo
publisher ownership. If you need a different name, update Cargo.toml, server.json,
binary/client examples if the executable changes, and relevant tests/docs.

With your authenticated crates.io account:

```sh
cargo login
cargo publish --dry-run --locked
cargo publish --locked
```

The dry run does not reserve the name or prove publisher permission. Confirm
the actual public package and version after publication. Only then can users
install the public crate with `cargo install project-memory-mcp --locked`.
Until then, use `cargo install --path . --locked` from the source checkout.

## 4. List the server in the MCP Registry

Follow the official [publishing guide](https://modelcontextprotocol.io/registry/quickstart)
and [Cargo requirements](https://modelcontextprotocol.io/registry/package-types).
The Registry hosts installation metadata, not executables or saved memories.
Cargo users install once, then their MCP client invokes the binary by name;
there is no Cargo equivalent of an npx runtime runner.

The visible README token is required for Cargo ownership verification:

`mcp-name: io.github.charliejamesgwapo/project-memory`

It must match `name` in server.json. Keep it visible: crates.io strips HTML
comments when rendering README content. Package name and version must match
the already-published crate. The checked-in server.json is a publishing template;
its namespace and proposed package identity still require live verification.

Use a current official mcp-publisher CLI:

```sh
mcp-publisher validate
mcp-publisher login github
mcp-publisher publish
```

Check `mcp-publisher --help` for the current command arguments. At build time,
the upstream Registry validator and Cargo documentation supported `cargo`,
while the public static schema URL returned a schema without that enum value.
The template was checked against the upstream validator schema. Revalidate
using the current publisher and live registry before release; the static schema
alone may lag. Verify GitHub namespace spelling/case with your authenticated
publisher account rather than assuming a local JSON check proves ownership.

Registry metadata is versioned. Change versions consistently, publish the new
crate, and then publish its corresponding registry metadata. Verify a live
listing before claiming the server is discoverable. Some clients use separate
catalogues; an official Registry listing does not guarantee automatic inclusion
in every AI application's catalogue.

## 5. Prepare native downloads

Pushing a version tag such as `v0.1.0`, or running the workflow manually, runs
the release-build workflow for Linux x64, macOS Apple Silicon, macOS Intel and
Windows x64. It uploads workflow artifacts; it does not create a public GitHub
Release or publish crates or Registry metadata.

After successful CI, create the GitHub Release and attach the appropriate
artifacts and checksums. Unix binaries are inside tar.gz files so executable
permissions survive GitHub's outer artifact ZIP; extract the inner archive before
running. Windows binaries are inside ZIP files. Signing/notarization is not configured. This local build
only verifies the current machine's platform; other platforms require successful
CI before their binaries should be advertised as tested.

## Costs

Public GitHub source and public crates.io distribution can be used without a
paid hosting service. Standard GitHub-hosted runners are free for public
repositories, subject to GitHub's current terms. The Registry publishing guide
documents no listing fee. Each user runs the binary on their own computer.
AI client subscriptions or model usage are separate from this server.
