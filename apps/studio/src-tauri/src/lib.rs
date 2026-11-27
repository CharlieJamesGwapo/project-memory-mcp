//! Native Project Memory Studio bridge.
#![forbid(unsafe_code)]

pub mod service;

use serde::Serialize;
use serde_json::Value;
use service::{ConnectionInfo, StudioError, StudioSession, validate_markdown, write_markdown};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::State;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;

type Session = Arc<Mutex<StudioSession>>;
type Result<T> = std::result::Result<T, StudioError>;

async fn with_session<T: Send + 'static>(
    session: Session,
    action: impl FnOnce(&mut StudioSession) -> Result<T> + Send + 'static,
) -> Result<T> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut session = session.lock().map_err(|_| {
            StudioError::new(
                "SESSION_ERROR",
                "The workspace could not be accessed. Restart Studio.",
            )
        })?;
        action(&mut session)
    })
    .await
    .map_err(|_| StudioError::new("SESSION_ERROR", "The workspace operation could not finish."))?
}

#[tauri::command(rename_all = "snake_case")]
async fn bootstrap(session: State<'_, Session>) -> Result<ConnectionInfo> {
    with_session(session.inner().clone(), |s| Ok(s.bootstrap())).await
}

#[tauri::command(rename_all = "snake_case")]
async fn choose_database(
    window: tauri::WebviewWindow,
    session: State<'_, Session>,
) -> Result<Option<ConnectionInfo>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Open Project Memory database")
        .pick_file(move |file| {
            let _ = tx.send(file);
        });
    let file = rx
        .await
        .map_err(|_| StudioError::new("DIALOG_ERROR", "The file picker could not finish."))?;
    let Some(file) = file else {
        return Ok(None);
    };
    let path = file
        .into_path()
        .map_err(|_| StudioError::new("INVALID_PATH", "Choose a local SQLite database file."))?;
    with_session(session.inner().clone(), move |s| {
        s.open_database(&path).map(Some)
    })
    .await
}

#[tauri::command(rename_all = "snake_case")]
async fn memory_query(
    session: State<'_, Session>,
    session_id: String,
    operation: String,
    args: Value,
) -> Result<Value> {
    with_session(session.inner().clone(), move |s| {
        s.query(&session_id, &operation, args)
    })
    .await
}

#[tauri::command(rename_all = "snake_case")]
fn copy_context(app: tauri::AppHandle, markdown: String) -> Result<()> {
    validate_markdown(&markdown)?;
    app.clipboard().write_text(markdown).map_err(|_| {
        StudioError::new(
            "CLIPBOARD_ERROR",
            "Could not copy the context brief. Try saving Markdown instead.",
        )
    })
}

#[derive(Serialize)]
struct ExportResult {
    saved: bool,
    file_name: Option<String>,
}

#[tauri::command(rename_all = "snake_case")]
async fn export_context(
    window: tauri::WebviewWindow,
    session: State<'_, Session>,
    markdown: String,
) -> Result<ExportResult> {
    validate_markdown(&markdown)?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Save context brief")
        .add_filter("Markdown", &["md"])
        .set_file_name("project-context.md")
        .save_file(move |file| {
            let _ = tx.send(file);
        });
    let file = rx
        .await
        .map_err(|_| StudioError::new("DIALOG_ERROR", "The save dialog could not finish."))?;
    let Some(file) = file else {
        return Ok(ExportResult {
            saved: false,
            file_name: None,
        });
    };
    let path = file
        .into_path()
        .map_err(|_| StudioError::new("INVALID_PATH", "Choose a local Markdown file."))?;
    with_session(session.inner().clone(), move |s| {
        write_markdown(&path, &markdown, s.database_path())?;
        Ok(ExportResult {
            saved: true,
            file_name: path.file_name().map(|s| s.to_string_lossy().into()),
        })
    })
    .await
}

fn app_context() -> tauri::Context<tauri::Wry> {
    tauri::generate_context!()
}

#[cfg(all(test, not(dev)))]
#[test]
fn production_assets_include_the_entrypoint_and_bundled_scripts() {
    let context = app_context();
    let index = context
        .assets()
        .get(&"index.html".into())
        .expect("The packaged app must contain index.html");
    let html = std::str::from_utf8(&index).expect("HTML is UTF-8");
    assert!(html.contains("Project Memory Studio"));
    for (prefix, suffix) in [("src=\"", ".js"), ("href=\"", ".css")] {
        let asset = html
            .split(prefix)
            .skip(1)
            .filter_map(|s| s.split('"').next())
            .find(|s| s.ends_with(suffix))
            .expect("Bundled script and stylesheet links exist");
        assert!(
            context.assets().get(&asset.into()).is_some(),
            "Missing bundled asset: {asset}"
        );
    }
}

pub fn run() {
    let default = std::env::var_os("PROJECT_MEMORY_DB")
        .map(PathBuf::from)
        .or_else(|| project_memory_mcp::paths::default_database_path().ok());
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(Arc::new(Mutex::new(StudioSession::new(default))))
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            choose_database,
            memory_query,
            copy_context,
            export_context
        ])
        .run(app_context())
        .expect("Could not start Project Memory Studio");
}
