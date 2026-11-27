use project_memory_mcp::{error::MemoryError, store::Store};
use serde::Serialize;
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize)]
pub struct StudioError {
    pub code: String,
    pub message: String,
}

impl StudioError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<MemoryError> for StudioError {
    fn from(error: MemoryError) -> Self {
        Self {
            code: error.code().into(),
            message: error.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionInfo {
    pub status: String,
    pub path: Option<String>,
    pub default_path: Option<String>,
    pub session_id: Option<String>,
    pub error: Option<String>,
}

pub struct StudioSession {
    store: Option<Store>,
    path: Option<PathBuf>,
    session_id: Option<String>,
    default_path: Option<PathBuf>,
}

impl StudioSession {
    pub fn new(default_path: Option<PathBuf>) -> Self {
        Self {
            store: None,
            path: None,
            session_id: None,
            default_path,
        }
    }

    fn info(&self, status: &str, error: Option<String>) -> ConnectionInfo {
        ConnectionInfo {
            status: status.into(),
            path: self.path.as_ref().map(|p| p.to_string_lossy().into()),
            default_path: self
                .default_path
                .as_ref()
                .map(|p| p.to_string_lossy().into()),
            session_id: self.session_id.clone(),
            error,
        }
    }

    pub fn bootstrap(&mut self) -> ConnectionInfo {
        if self.store.is_some() {
            return self.info("connected", None);
        }
        let Some(path) = self.default_path.clone() else {
            return self.info("missing", None);
        };
        if !path.exists() {
            return self.info("missing", None);
        }
        match self.open_database(&path) {
            Ok(info) => info,
            Err(error) => self.info("error", Some(error.message)),
        }
    }

    pub fn open_database(&mut self, path: &Path) -> Result<ConnectionInfo, StudioError> {
        // Construct before replacing: a failed selection retains the previous session.
        let store = Store::open_read_only(path)?;
        self.store = Some(store);
        self.path = Some(path.to_path_buf());
        self.session_id = Some(uuid::Uuid::new_v4().to_string());
        Ok(self.info("connected", None))
    }

    pub fn database_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn query(
        &mut self,
        session_id: &str,
        operation: &str,
        args: Value,
    ) -> Result<Value, StudioError> {
        if !matches!(
            operation,
            "studio_list_projects"
                | "studio_list_records"
                | "list_projects"
                | "get_project_context"
                | "get_record"
                | "search_notes"
                | "search_memory"
                | "get_context_bundle"
                | "export_project"
        ) {
            return Err(StudioError::new(
                "OPERATION_NOT_ALLOWED",
                "Studio only permits read operations.",
            ));
        }
        if self.store.is_none() {
            return Err(StudioError::new(
                "NO_DATABASE",
                "Open a Project Memory database first.",
            ));
        }
        if self.session_id.as_deref() != Some(session_id) {
            return Err(StudioError::new(
                "STALE_SESSION",
                "The database changed. Refresh the current workspace.",
            ));
        }
        if serde_json::to_vec(&args).map_or(true, |v| v.len() > 65_536) {
            return Err(StudioError::new(
                "VALIDATION_ERROR",
                "Query arguments are too large.",
            ));
        }
        self.store
            .as_mut()
            .unwrap()
            .execute(operation, args)
            .map_err(Into::into)
    }
}

pub fn validate_markdown(markdown: &str) -> Result<(), StudioError> {
    if markdown.trim().is_empty() {
        return Err(StudioError::new(
            "EMPTY_CONTEXT",
            "Select records before exporting context.",
        ));
    }
    if markdown.len() > 2 * 1024 * 1024 {
        return Err(StudioError::new(
            "CONTEXT_TOO_LARGE",
            "Context exceeds the 2 MiB export limit. Remove some records.",
        ));
    }
    Ok(())
}

pub fn write_markdown(
    path: &Path,
    markdown: &str,
    database: Option<&Path>,
) -> Result<(), StudioError> {
    validate_markdown(markdown)?;
    let invalid = || {
        StudioError::new(
            "INVALID_DESTINATION",
            "Choose a Markdown (.md) file outside the memory database.",
        )
    };
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
    {
        return Err(invalid());
    }
    let io_error = |_| {
        StudioError::new(
            "EXPORT_ERROR",
            "Could not save Markdown. Check the destination and file permissions.",
        )
    };
    let existing = match fs::symlink_metadata(path) {
        Ok(meta) => {
            if !meta.file_type().is_file() {
                return Err(invalid());
            }
            if let Some(db) = database
                && fs::canonicalize(path).ok() == fs::canonicalize(db).ok()
            {
                return Err(invalid());
            }
            let mut header = [0; 16];
            let mut file = fs::File::open(path).map_err(io_error)?;
            if file.read(&mut header).map_err(io_error)? == 16 && &header == b"SQLite format 3\0" {
                return Err(invalid());
            }
            Some(meta.permissions())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(io_error(e)),
    };
    let parent = path.parent().ok_or_else(invalid)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    temp.write_all(markdown.as_bytes()).map_err(io_error)?;
    if let Some(permissions) = existing {
        temp.as_file()
            .set_permissions(permissions)
            .map_err(io_error)?;
    }
    temp.as_file().sync_all().map_err(io_error)?;
    temp.persist(path).map_err(|_| {
        StudioError::new(
            "EXPORT_ERROR",
            "Could not replace the chosen Markdown file.",
        )
    })?;
    Ok(())
}
