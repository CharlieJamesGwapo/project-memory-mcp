//! Read-only setup diagnostics and exact client configuration previews.
use crate::{
    error::{MemoryError, Result},
    security::{AccessPolicy, validate_audit_destination},
    store::Store,
};
use serde_json::{Value, json};
use std::{fs, path::Path};

#[derive(Clone, Copy)]
enum PathState {
    Existing,
    Missing,
    Invalid,
}

// Match Store's final-component symlink rule. Parent aliases such as /var on
// macOS remain usable; audit/archive alias comparisons resolve existing parents.
fn inspect_path(path: &Path) -> PathState {
    if !path.is_absolute() || path.to_str().is_none() {
        return PathState::Invalid;
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => PathState::Existing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => PathState::Missing,
        _ => PathState::Invalid,
    }
}

#[derive(Default)]
struct Checks {
    items: Vec<Value>,
    actions: Vec<&'static str>,
}

impl Checks {
    fn passed(&mut self, code: &str, message: &str) {
        self.items
            .push(json!({"code":code,"status":"passed","message":message}));
    }

    fn action(&mut self, code: &str, message: &str, action: &'static str) {
        self.items
            .push(json!({"code":code,"status":"action_required","message":message}));
        self.actions.push(action);
    }
}

/// Inspect explicit local setup inputs without executing a server, initializing
/// an archive, opening an audit log, or changing another client's settings.
/// Expected setup problems are returned as actionable report checks.
pub fn preview(
    database: &Path,
    executable: &Path,
    policy: &AccessPolicy,
    policy_path: Option<&Path>,
    read_only: bool,
) -> Result<Value> {
    let mut checks = Checks::default();
    let database_state = inspect_path(database);
    let database_safe = !matches!(database_state, PathState::Invalid);
    if database_safe {
        checks.passed("database_path", "The database path is absolute and is not an existing directory or final-component symlink.");
    } else {
        checks.action("database_path_invalid", "The database path must be an accessible absolute UTF-8 file path, not a directory or final-component symlink.", "Choose an absolute database file path with accessible parent directories.");
    }

    let executable_safe = match inspect_path(executable) {
        PathState::Existing => {
            checks.passed("executable_metadata", "The executable path names an existing absolute regular file. Only filesystem metadata was inspected; the file was not executed.");
            true
        }
        PathState::Missing => {
            checks.action(
                "executable_missing",
                "No regular server executable exists at the supplied absolute path.",
                "Install or build the native server, then enter its absolute executable path.",
            );
            false
        }
        PathState::Invalid => {
            checks.action("executable_invalid", "The executable must be an accessible absolute UTF-8 regular file path, not a directory or final-component symlink.", "Choose the absolute path of the regular server executable file.");
            false
        }
    };

    let policy_valid = policy.validate().is_ok();
    let mut policy_reproducible = policy_valid;
    if policy_valid {
        checks.passed("policy_configuration", "The supplied local policy configuration passed validation. This does not test its audit destination.");
    } else {
        checks.action("policy_invalid", "The supplied local policy contains invalid project IDs, operations, or an audit path.", "Correct the local policy using the documented project IDs, supported operations, and absolute audit path.");
    }
    if let Some(path) = policy_path {
        if !matches!(inspect_path(path), PathState::Existing) {
            policy_reproducible = false;
            checks.action("policy_file_invalid", "The policy path must name an existing absolute UTF-8 regular file, not a directory or final-component symlink.", "Choose an existing absolute local policy JSON path.");
        } else {
            match AccessPolicy::load(path) {
                Ok(loaded) if json!(loaded) == json!(policy) => checks.passed("policy_file", "The policy file matches the supplied policy and will be preserved in client arguments."),
                _ => {
                    policy_reproducible = false;
                    checks.action("policy_file_mismatch", "The policy file is invalid or differs from the supplied policy; copying configuration could change restrictions.", "Load the intended local policy file again before copying client configuration.");
                }
            }
        }
    } else if json!(policy) != json!(AccessPolicy::default()) {
        policy_reproducible = false;
        checks.action("policy_file_required", "A nondefault policy needs an explicit policy file so copied client configuration preserves its restrictions.", "Save the intended policy separately and supply its absolute policy path before copying configuration.");
    }

    if let Some(audit) = &policy.audit_path {
        if database_safe && policy_valid && validate_audit_destination(database, audit).is_err() {
            policy_reproducible = false;
            checks.action(
                "audit_archive_alias",
                "The audit destination must differ from the archive and its SQLite sidecars.",
                "Choose an audit destination separate from the archive and its SQLite sidecars.",
            );
        }
        checks.passed("audit_not_tested", "Audit is configured. Preview did not open or append the audit log and did not test its writability or existing file header.");
    }

    let mut diagnostics = Value::Null;
    match database_state {
        PathState::Missing => checks.action("archive_missing", "No Project Memory archive exists at this path. Preview has not created an archive or parent directories.", if read_only { "Initialize an archive explicitly in writable mode or choose an existing supported archive before using this read-only configuration." } else { "Run init explicitly with the intended database path, or let a writable MCP client initialize it on first connection." }),
        PathState::Existing if policy_valid => {
            // set_policy normally opens audit destinations. Remove only audit
            // handling from this local clone while retaining project scope;
            // report the original configured policy below.
            let inspected = Store::open_read_only(database).and_then(|mut store| {
                let mut inspection_policy = policy.clone();
                inspection_policy.audit_path = None;
                store.set_policy(inspection_policy)?;
                store.diagnostics()
            });
            match inspected {
                Ok(value) => {
                    diagnostics = value;
                    checks.passed("archive_readable", "The existing supported archive passed a read-only SQLite integrity check. Counts reflect the supplied project scope.");
                }
                Err(error) => {
                    let (code, message, action) = match error {
                        MemoryError::ForeignDatabase => ("archive_foreign", "The file has a foreign or unsupported archive identity.", "Choose a supported Project Memory archive or a separate new database path; preview does not adopt or repair this file."),
                        MemoryError::NewerSchema => ("archive_newer_schema", "The archive schema is newer than this application supports.", "Upgrade the server before using this archive; preview does not migrate it."),
                        MemoryError::Integrity => ("archive_integrity", "The archive did not pass the SQLite integrity check.", "Inspect a separate copy or restore a trusted backup; preview does not repair the archive."),
                        _ => ("archive_unreadable", "The archive could not be inspected read-only as a supported Project Memory archive.", "Check the selected archive, local file permissions, and server version; preview does not repair the archive."),
                    };
                    checks.action(code, message, action);
                }
            }
        }
        _ => {}
    }

    let configuration = if database_safe && executable_safe && policy_reproducible {
        let mut args = vec![json!("--db"), json!(database)];
        if let Some(path) = policy_path {
            args.extend([json!("--policy"), json!(path)]);
        }
        if read_only {
            args.push(json!("--read-only"));
        }
        json!({"mcpServers":{"project-memory":{"command":executable,"args":args}}})
    } else {
        Value::Null
    };
    checks.passed("connection_not_tested", "The server process was not started. Executable authenticity, version, MCP handshake, and client compatibility were not tested.");

    Ok(json!({
        "status": if checks.actions.is_empty() { "ready" } else { "action_required" },
        "checks":checks.items,
        "configuration":configuration,
        "policy":if policy_valid { policy.summary(read_only) } else { Value::Null },
        "diagnostics":diagnostics,
        "next_actions":checks.actions,
        "notice":"Setup preview only: server execution, executable authenticity/version, MCP connection and client compatibility were not tested. No archive initialization, audit log opening, or client settings changes were performed. SQLite read-only inspection may maintain WAL/SHM coordination files; record and schema contents remain unchanged. Ready means these local checks passed, not that a client connection or audit destination is verified."
    }))
}
