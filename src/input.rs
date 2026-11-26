use crate::error::{MemoryError, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

fn default_limit() -> u32 {
    20
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateProject {
    /// Project name, 1–200 characters. Names need not be unique; use returned IDs.
    pub name: String,
    /// Project purpose, 1–4000 characters.
    pub purpose: String,
    /// Optional directory reference; never scanned or opened by this server.
    #[serde(default)]
    pub directory: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListProjects {
    /// Page size: 1–50; default 20.
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectId {
    pub project_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub project_id: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub cursor: Option<String>,
    /// Include superseded decisions in the paginated history. Default false.
    #[serde(default)]
    pub include_superseded: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordId {
    pub project_id: String,
    pub record_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Author {
    User,
    Assistant,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub project_id: String,
    pub title: String,
    /// Literal note content, max 16000 Unicode scalar values.
    pub content: String,
    /// Who originated the content. Assistant inferences must use assistant.
    pub author_type: Author,
    /// References only, not automatically verified evidence. Max 10, each 2048 characters.
    #[serde(default)]
    pub sources: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Search {
    pub project_id: String,
    /// Literal substring of note title/content, including NUL; no wildcard syntax.
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub project_id: String,
    pub decision: String,
    pub rationale: String,
    pub author_type: Author,
    #[serde(default)]
    pub sources: Vec<String>,
    /// ID of an active decision in this project that this explicitly replaces.
    #[serde(default)]
    pub supersedes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Handoff {
    pub project_id: String,
    pub summary: String,
    /// Each list has at most 20 entries; each entry at most 1000 characters.
    pub completed: Vec<String>,
    pub unfinished: Vec<String>,
    pub next_actions: Vec<String>,
    pub blockers: Vec<String>,
    /// Actual commands/results or explicitly unverified checks; never invented evidence.
    pub verification: Vec<String>,
    pub author_type: Author,
    #[serde(default)]
    pub sources: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Export {
    pub project_id: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteProject {
    pub project_id: String,
    /// Must be true. Only invoke after a direct user request to delete this project.
    pub confirm: bool,
}

pub fn parse<T: DeserializeOwned>(input: Value) -> Result<T> {
    serde_json::from_value(input).map_err(|e| MemoryError::Validation(e.to_string()))
}
pub fn text(value: &str, field: &str, max: usize) -> Result<()> {
    if value.trim().is_empty() || value.chars().count() > max {
        return Err(MemoryError::Validation(format!(
            "{field} must contain 1–{max} characters"
        )));
    }
    Ok(())
}
pub fn id(value: &str) -> Result<()> {
    if uuid::Uuid::parse_str(value).is_err()
        || value.len() != 36
        || value != value.to_ascii_lowercase()
    {
        return Err(MemoryError::Validation(
            "IDs must be lowercase, hyphenated UUIDs".into(),
        ));
    }
    Ok(())
}
pub fn page(limit: u32) -> Result<()> {
    if !(1..=50).contains(&limit) {
        return Err(MemoryError::Validation("limit must be 1–50".into()));
    }
    Ok(())
}
pub fn list(values: &[String], field: &str, max_count: usize, max_length: usize) -> Result<()> {
    if values.len() > max_count {
        return Err(MemoryError::Validation(format!(
            "{field} has too many entries (max {max_count})"
        )));
    }
    for value in values {
        text(value, field, max_length)?;
    }
    Ok(())
}
pub fn record_size(value: &Value) -> Result<()> {
    if value.to_string().len() > 65536 {
        return Err(MemoryError::Validation(
            "record input exceeds 65536 UTF-8 bytes".into(),
        ));
    }
    Ok(())
}
