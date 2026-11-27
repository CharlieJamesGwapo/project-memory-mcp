use project_memory_mcp::store::Store;
use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Command, Output},
};

fn cli(path: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"))
        .arg("--db")
        .arg(path)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn doctor_missing_archive_is_actionable_and_never_creates_it() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("not-created/memory.sqlite");
    let out = cli(&path, &["doctor", "--json"]);
    assert_eq!(out.status.code(), Some(2));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["status"], "missing");
    assert!(
        value["next_action"]
            .as_str()
            .unwrap()
            .contains("MCP client")
    );
    assert!(out.stderr.is_empty());
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn doctor_checks_existing_archives_without_modifying_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("memory.sqlite");
    let mut store = Store::open(&path).unwrap();
    store
        .execute(
            "create_project",
            json!({"name":"Archive","purpose":"Doctor test"}),
        )
        .unwrap();
    drop(store);
    let before = std::fs::read(&path).unwrap();
    let out = cli(&path, &["doctor", "--json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["status"], "ready");
    assert_eq!(value["diagnostics"]["schema_version"], 1);
    assert_eq!(value["diagnostics"]["integrity"], "ok");
    assert_eq!(value["diagnostics"]["project_count"], 1);
    assert_eq!(value["diagnostics"]["record_count"], 0);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn doctor_refuses_foreign_files_without_starting_or_mutating_them() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("foreign.sqlite");
    std::fs::write(&path, b"not a memory database").unwrap();
    let out = cli(&path, &["doctor", "--json"]);
    assert_eq!(out.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["status"], "error");
    assert!(value["message"].is_string());
    assert_eq!(std::fs::read(&path).unwrap(), b"not a memory database");
}

#[test]
fn print_config_is_valid_json_with_absolute_paths_and_no_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("a path with spaces/memory.sqlite");
    let out = cli(&path, &["--read-only", "print-config"]);
    assert!(out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    let server = &value["mcpServers"]["project-memory"];
    assert!(Path::new(server["command"].as_str().unwrap()).is_absolute());
    assert_eq!(
        server["args"],
        json!(["--db", path.to_string_lossy(), "--read-only"])
    );
    assert!(out.stderr.is_empty());
    assert!(!path.parent().unwrap().exists());
}
