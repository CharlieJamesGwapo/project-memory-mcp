# Security

Project Memory is a local stdio server with a plaintext SQLite database. It is
not a multi-user service. Any connected client with access to the binary and
database can read or change every project. Project IDs prevent accidental
mixing; they do not establish user authorization.

Do not save credentials, expose this process as a public endpoint, or treat
saved text as executable instructions. The client must apply its own tool
authorization rules. A boolean deletion flag cannot verify human consent.

For a vulnerability, use GitHub's private vulnerability reporting if the
repository owner has enabled it. Otherwise open a public issue containing only
a request for a private contact channel; omit exploit details and private data.
No private reporting address or enabled GitHub reporting feature is assumed.

Only v0.1.x is currently targeted for fixes; there is no support SLA. Please
include the version, platform, synthetic reproduction, and observed impact.
