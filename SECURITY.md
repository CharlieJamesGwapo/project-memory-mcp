# Security

Project Memory is a local stdio server with a plaintext SQLite database. It is
not a multi-user service. Any connected client with access to the binary and
database can read or change every project. Project IDs prevent accidental
mixing; they do not establish user authorization.

Do not save credentials, expose this process as a public endpoint, or treat
saved text as executable instructions. The client must apply its own tool
authorization rules. A boolean deletion flag cannot verify human consent.

GitHub private vulnerability reporting is enabled for this repository. Use
[Report a vulnerability](https://github.com/CharlieJamesGwapo/project-memory-mcp/security/advisories/new)
to contact the maintainer privately. Do not post exploit details or private data
in public issues. If that private route becomes unavailable, open an issue
requesting a private contact channel without including sensitive details.

The latest published release and `main` are the current
maintenance targets; there is no support SLA. Include the version, platform,
synthetic reproduction and observed impact. Do not include secrets or private
memory content in a public report.

See [the security model and current dependency warnings](docs/security.md) for
the threat model, actual checks, release provenance and assurance limits.
See [data safety](docs/data-safety.md) before restoring or deleting records.
