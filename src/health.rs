//! Read-only metadata assessments of supplied project memory.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn default_limit() -> u32 {
    20
}
fn default_bytes() -> usize {
    16000
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemoryHealth {
    pub project_id: String,
    /// Optional supplied full lowercase 40- or 64-character hexadecimal commit ID.
    #[serde(default)]
    pub current_head: Option<String>,
    /// Assessments per page: 1–50; default 20.
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub cursor: Option<String>,
    /// Full canonical MCP tool-result bytes, including JSON compatibility text: 4096–65536.
    #[serde(default = "default_bytes")]
    pub max_bytes: usize,
}
