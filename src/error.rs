use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Invalid input: {0}")]
    Validation(String),
    #[error("Project or record not found in the requested project")]
    NotFound,
    #[error("The target decision is already superseded or is not a decision")]
    Conflict,
    #[error("Cursor is invalid or belongs to a different query")]
    InvalidCursor,
    #[error("Database operation failed; check the configured database and file permissions")]
    Storage(#[from] rusqlite::Error),
    #[error("Cannot access the configured database file")]
    Io(#[from] std::io::Error),
    #[error("Database schema is newer than this application; upgrade the application")]
    NewerSchema,
}

impl MemoryError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::InvalidCursor => "INVALID_CURSOR",
            Self::Storage(_) | Self::Io(_) => "STORAGE_ERROR",
            Self::NewerSchema => "NEWER_SCHEMA",
        }
    }
    pub fn payload(&self) -> serde_json::Value {
        #[derive(Serialize)]
        struct Body<'a> { code: &'a str, message: String }
        serde_json::json!({"error": Body {code: self.code(), message: self.to_string()}})
    }
}

pub type Result<T> = std::result::Result<T, MemoryError>;
