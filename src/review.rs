//! Bounded review evidence. These values describe supplied history, not server attestation.
use crate::{
    error::{MemoryError, Result},
    input,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckOutcome {
    Passed,
    Failed,
    NotRun,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewCheck {
    pub name: String,
    /// The author's reported outcome. The server never executes or certifies checks.
    pub outcome: CheckOutcome,
    pub result: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Informational,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Open,
    Fixed,
    Accepted,
    NeedsVerification,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewFinding {
    /// Unique within this review; preserve it when following the same issue in a revision.
    pub id: String,
    pub severity: Severity,
    pub title: String,
    /// Safe relative forward-slash reference; this path is never opened.
    pub path: String,
    pub line: u32,
    pub evidence: String,
    pub correction: String,
    pub status: FindingStatus,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveReview {
    pub project_id: String,
    /// Repository label only; no repository is opened, cloned or verified.
    pub repository: String,
    /// Full lowercase hexadecimal commit ID: 40 or 64 characters.
    pub base_commit: String,
    pub head_commit: String,
    pub summary: String,
    pub checks: Vec<ReviewCheck>,
    pub findings: Vec<ReviewFinding>,
    pub author_type: input::Author,
    #[serde(default)]
    pub sources: Vec<String>,
    /// Append a successor to this review in the same project/repository, without editing it.
    #[serde(default)]
    pub previous_review_id: Option<String>,
}

fn default_limit() -> u32 {
    10
}
fn default_bytes() -> usize {
    16000
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewHistory {
    pub project_id: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub cursor: Option<String>,
    /// Compare supplied head with saved heads. A match is not a verification verdict.
    #[serde(default)]
    pub current_head: Option<String>,
    /// Full canonical MCP result bytes, including the text compatibility copy: 4096–65536.
    #[serde(default = "default_bytes")]
    pub max_bytes: usize,
}

pub(crate) fn commit(value: &str) -> Result<()> {
    if !matches!(value.len(), 40 | 64)
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(MemoryError::Validation(
            "commit IDs must be full lowercase 40- or 64-character hexadecimal values".into(),
        ));
    }
    Ok(())
}

impl SaveReview {
    pub(crate) fn validate(&self) -> Result<()> {
        input::id(&self.project_id)?;
        input::text(&self.repository, "repository", 512)?;
        commit(&self.base_commit)?;
        commit(&self.head_commit)?;
        input::text(&self.summary, "summary", 2000)?;
        input::list(&self.sources, "sources", 10, 2048)?;
        if let Some(id) = &self.previous_review_id {
            input::id(id)?;
        }
        if self.checks.len() > 20 || self.findings.len() > 20 {
            return Err(MemoryError::Validation(
                "reviews allow at most 20 checks and 20 findings".into(),
            ));
        }
        for check in &self.checks {
            input::text(&check.name, "check name", 200)?;
            input::text(&check.result, "check result", 2000)?;
        }
        let mut ids = HashSet::new();
        for finding in &self.findings {
            if finding.id.is_empty()
                || finding.id.len() > 64
                || !finding
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
                || !ids.insert(&finding.id)
            {
                return Err(MemoryError::Validation("finding IDs must be unique 1–64-character ASCII letters, digits, underscore or hyphen".into()));
            }
            input::text(&finding.title, "finding title", 200)?;
            input::text(&finding.path, "finding path", 1024)?;
            if finding.path.contains(['\\', ':'])
                || finding.path.chars().any(char::is_control)
                || finding
                    .path
                    .split('/')
                    .any(|part| part.is_empty() || matches!(part, "." | ".."))
            {
                return Err(MemoryError::Validation(
                    "finding paths must use safe relative forward-slash components".into(),
                ));
            }
            if !(1..=1000000).contains(&finding.line) {
                return Err(MemoryError::Validation(
                    "finding line must be 1–1000000".into(),
                ));
            }
            input::text(&finding.evidence, "finding evidence", 2000)?;
            input::text(&finding.correction, "finding correction", 2000)?;
        }
        Ok(())
    }
}
