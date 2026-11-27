use crate::error::{MemoryError, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

/// The MCP CLI and Studio share the same local memory library by default.
pub fn default_database_path() -> Result<PathBuf> {
    ProjectDirs::from("", "", "project-memory-mcp")
        .map(|directories| directories.data_local_dir().join("memory.sqlite"))
        .ok_or_else(|| {
            MemoryError::Validation(
                "Cannot find user data directory; choose an absolute database path".into(),
            )
        })
}
