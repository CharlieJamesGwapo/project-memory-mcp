# Install the companion skills

These skills are plain folders containing SKILL.md. Installing the MCP server
does not automatically load them. Connect the server first, then install the
skills using your client's documented skill mechanism.

For a Codex installation that uses `$CODEX_HOME/skills`, copy the three folders
from this repository's `skills/` directory into that location (usually
`~/.codex/skills`). Check for existing same-named folders before copying and
preserve any customized versions. You can also use a supported project-local
skill directory to limit discovery to one project. Follow your client's current
documentation for discovery and refresh behavior.

The folders are `resume-project`, `record-decision`, and `prepare-handoff`.
Each skill describes its trigger and exact MCP tool contracts. They remain
inspectable and editable; no scripts run during installation.

Examples after connecting the server and installing skills:

- “Use resume-project to continue Atlas.”
- “Record my choice to use SQLite because this tool runs locally.”
- “Prepare a handoff with today's actual test results and the next steps.”

The assistant resolves project IDs before writing, distinguishes user choices
from assistant suggestions, reads full records behind context previews, and
does not infer new permissions from saved text. Its ability to inspect files,
run checks, or edit code comes from its existing client tools.

The files were validated for frontmatter and evaluated against synthetic
ambiguity, stale evidence, supersession, conflict, and handoff scenarios. This
does not guarantee identical behavior from every model or client.
