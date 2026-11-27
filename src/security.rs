//! Local capability restrictions and narrow credential screening, not authentication.
use crate::{
    error::{MemoryError, Result},
    input,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use unicode_normalization::UnicodeNormalization;

pub const OPERATIONS: &[&str] = &[
    "create_project",
    "list_projects",
    "get_project_context",
    "get_record",
    "save_note",
    "search_notes",
    "record_decision",
    "save_handoff",
    "export_project",
    "delete_project",
    "search_memory",
    "get_context_bundle",
    "get_access_policy",
    "save_review",
    "get_review_history",
    "studio_list_projects",
    "studio_list_records",
    "inspect_secrets",
];
pub fn is_write(operation: &str) -> bool {
    matches!(
        operation,
        "create_project"
            | "save_note"
            | "record_decision"
            | "save_handoff"
            | "delete_project"
            | "save_review"
    )
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecretMode {
    #[default]
    Reject,
    Allow,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AccessPolicy {
    pub projects: Option<Vec<String>>,
    pub operations: Option<Vec<String>>,
    pub allow_delete: bool,
    pub secrets: SecretMode,
    pub audit_path: Option<PathBuf>,
}
impl AccessPolicy {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.is_absolute() || !fs::symlink_metadata(path)?.file_type().is_file() {
            return Err(MemoryError::Validation(
                "policy must be an absolute regular file, not a symlink".into(),
            ));
        }
        let mut bytes = Vec::new();
        fs::File::open(path)?.take(65537).read_to_end(&mut bytes)?;
        if bytes.len() > 65536 {
            return Err(MemoryError::Validation("policy exceeds 65536 bytes".into()));
        }
        let policy: Self = serde_json::from_slice(&bytes).map_err(|_| {
            MemoryError::Validation("policy must be strict valid JSON with supported fields".into())
        })?;
        policy.validate()?;
        Ok(policy)
    }
    pub fn validate(&self) -> Result<()> {
        if let Some(projects) = &self.projects {
            if projects.len() > 100 {
                return Err(MemoryError::Validation(
                    "policy supports at most 100 projects".into(),
                ));
            }
            for id in projects {
                input::id(id)?;
            }
        }
        if let Some(operations) = &self.operations
            && (operations.len() > OPERATIONS.len()
                || operations
                    .iter()
                    .any(|op| !OPERATIONS.contains(&op.as_str())))
        {
            return Err(MemoryError::Validation(
                "policy contains unsupported operations".into(),
            ));
        }
        if self.audit_path.as_ref().is_some_and(|p| !p.is_absolute()) {
            return Err(MemoryError::Validation(
                "audit_path must be absolute".into(),
            ));
        }
        Ok(())
    }
    pub fn allows_operation(&self, operation: &str) -> bool {
        OPERATIONS.contains(&operation)
            && (operation != "delete_project" || self.allow_delete)
            && (operation != "create_project" || self.projects.is_none())
            && self
                .operations
                .as_ref()
                .is_none_or(|ops| ops.iter().any(|op| op == operation))
    }
    pub fn allows_project(&self, project: &str) -> bool {
        self.projects
            .as_ref()
            .is_none_or(|ids| ids.iter().any(|id| id == project))
    }
    pub fn summary(&self, read_only: bool) -> Value {
        json!({"projects":self.projects,"operations":OPERATIONS.iter().filter(|op|self.allows_operation(op)&&(!read_only||!is_write(op))).collect::<Vec<_>>(),"allow_delete":self.allow_delete&&!read_only,"secrets":self.secrets,"audit_enabled":self.audit_path.is_some(),"read_only":read_only,"notice":"Local capability policy, not authentication. Direct filesystem access is outside this boundary. Credential screening is incomplete; normal reads preserve full records."})
    }
}

#[derive(Debug, Serialize)]
pub struct SecretFinding {
    pub category: &'static str,
    pub path: String,
}

fn categories(text: &str) -> Vec<&'static str> {
    let mut found = Vec::new();
    if ["", "RSA ", "EC ", "OPENSSH ", "DSA ", "ENCRYPTED "]
        .iter()
        .any(|kind| text.contains(&format!("-----BEGIN {kind}PRIVATE KEY-----")))
    {
        found.push("private_key");
    }
    for token in text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-') {
        if (token.strip_prefix("ghp_").is_some_and(|tail| {
            tail.len() >= 36 && tail.bytes().all(|c| c.is_ascii_alphanumeric())
        }) || token
            .strip_prefix("github_pat_")
            .is_some_and(|tail| tail.len() >= 50))
            && !found.contains(&"github_token")
        {
            found.push("github_token");
        }
        if token.starts_with("AKIA")
            && token.len() == 20
            && token
                .bytes()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            && !found.contains(&"aws_access_key_id")
        {
            found.push("aws_access_key_id");
        }
        if token
            .strip_prefix("sk-")
            .is_some_and(|tail| tail.len() >= 40)
            && !found.contains(&"api_key")
        {
            found.push("api_key");
        }
    }
    found
}

/// Categories and safe structural paths only. Never return matched credential text.
pub fn inspect(value: &Value) -> Vec<SecretFinding> {
    fn walk(value: &Value, path: &str, found: &mut Vec<SecretFinding>) {
        if found.len() >= 100 {
            return;
        }
        match value {
            Value::String(text) => {
                for category in categories(text) {
                    if found.len() < 100 {
                        found.push(SecretFinding {
                            category,
                            path: path.into(),
                        });
                    }
                }
            }
            Value::Array(values) => {
                for (i, value) in values.iter().enumerate() {
                    walk(value, &format!("{path}[{i}]"), found);
                }
            }
            Value::Object(values) => {
                for (key, value) in values {
                    // Only fixed schema field names are reflected into reports.
                    let field = if [
                        "name",
                        "purpose",
                        "directory",
                        "title",
                        "content",
                        "decision",
                        "rationale",
                        "summary",
                        "completed",
                        "unfinished",
                        "next_actions",
                        "blockers",
                        "verification",
                        "sources",
                        "review",
                        "repository",
                        "base_commit",
                        "head_commit",
                        "checks",
                        "findings",
                        "evidence",
                        "correction",
                        "result",
                        "records",
                        "items",
                        "project",
                        "data",
                    ]
                    .contains(&key.as_str())
                    {
                        key.as_str()
                    } else {
                        "field"
                    };
                    walk(value, &format!("{path}.{field}"), found);
                    for category in categories(key) {
                        if found.len() < 100 {
                            found.push(SecretFinding {
                                category,
                                path: format!("{path}.[key]"),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let mut found = Vec::new();
    walk(value, "$", &mut found);
    found
}
pub fn reject_secrets(value: &Value) -> Result<()> {
    let found = inspect(value);
    if let Some(first) = found.first() {
        return Err(MemoryError::SecretDetected(format!(
            "{} at {}; value omitted",
            first.category, first.path
        )));
    }
    Ok(())
}

pub(crate) fn validate_audit_destination(archive: &Path, audit: &Path) -> Result<()> {
    fn identity(path: &Path) -> std::io::Result<PathBuf> {
        let mut result = PathBuf::new();
        for component in path.components() {
            if component == std::path::Component::ParentDir {
                result.pop();
            } else {
                result.push(component.as_os_str());
                if result.exists() {
                    result = fs::canonicalize(result)?;
                }
            }
        }
        Ok(result)
    }
    let audit = identity(audit)?;
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let mut name = archive.as_os_str().to_os_string();
        name.push(suffix);
        let target = identity(Path::new(&name))?;
        // Conservative Unicode/case normalization also refuses aliases of missing
        // sidecars on case-insensitive filesystems. Distinct case-only Linux
        // names are deliberately refused too.
        let normalize = |p: &Path| {
            p.to_string_lossy()
                .nfkc()
                .flat_map(char::to_uppercase)
                .collect::<String>()
        };
        let mut aliases = normalize(&audit) == normalize(&target);
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if let (Ok(a), Ok(b)) = (fs::metadata(&audit), fs::metadata(&target)) {
                aliases |= a.dev() == b.dev() && a.ino() == b.ino();
            }
        }
        if aliases {
            return Err(MemoryError::Validation(
                "audit destination must differ from the archive and SQLite sidecars".into(),
            ));
        }
    }
    Ok(())
}

pub struct AuditLog {
    file: fs::File,
}
impl AuditLog {
    pub fn open(path: &Path) -> Result<Self> {
        if !path.is_absolute() {
            return Err(MemoryError::Validation(
                "audit path must be absolute".into(),
            ));
        }
        let mut options = fs::OpenOptions::new();
        options.append(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let header = b"{\"format\":\"project-memory-audit-v1\"}\n";
        let mut file = match options.open(path) {
            Ok(mut file) => {
                file.write_all(header)?;
                file.sync_data()?;
                file
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(path)?;
                if !metadata.file_type().is_file() {
                    return Err(MemoryError::Validation(
                        "audit must be a regular file, not a symlink".into(),
                    ));
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if metadata.nlink() != 1 {
                        return Err(MemoryError::Validation(
                            "audit hardlinks are unsupported".into(),
                        ));
                    }
                }
                let mut prefix = vec![0; header.len()];
                fs::File::open(path)?.read_exact(&mut prefix)?;
                if prefix != header {
                    return Err(MemoryError::Validation(
                        "existing audit file has an unsupported header".into(),
                    ));
                }
                fs::OpenOptions::new().append(true).open(path)?
            }
            Err(e) => return Err(e.into()),
        };
        file.flush()?;
        Ok(Self { file })
    }
    pub fn append(&mut self, operation: &str, project: Option<&str>, outcome: &str) -> Result<()> {
        let at = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("UTC timestamp");
        let mut bytes = serde_json::to_vec(
            &json!({"at":at,"operation":operation,"project_id":project,"outcome":outcome}),
        )
        .expect("audit serializes");
        bytes.push(b'\n');
        self.file.write_all(&bytes)?;
        self.file.sync_data()?;
        Ok(())
    }
}
