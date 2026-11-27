use project_memory_mcp::store::Store;
use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Command, Output},
};
fn cli(db: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"))
        .arg("--db")
        .arg(db)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn setup_missing_archive_returns_actionable_preview_without_creating_paths() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("missing folder/memory.sqlite");
    let result = cli(&db, &["--read-only", "setup", "--json"]);
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stderr.is_empty());
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["status"], "action_required");
    assert!(!report["configuration"].is_null());
    assert!(!db.parent().unwrap().exists());
    let args = report["configuration"]["mcpServers"]["project-memory"]["args"]
        .as_array()
        .unwrap();
    assert!(args.contains(&json!("--read-only")));
    assert!(report["notice"].as_str().unwrap().contains("not tested"));
}

#[test]
fn setup_preserves_policy_without_creating_an_audit_log_and_health_is_scoped_readonly() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("memory.sqlite");
    let mut store = Store::open(&db).unwrap();
    let p = store
        .execute(
            "create_project",
            json!({"name":"Guided","purpose":"Synthetic CLI"}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    store.execute("save_note",json!({"project_id":p,"title":"Evidence","content":"No source references yet","author_type":"user"})).unwrap();
    drop(store);
    let audit = temp.path().join("audit.jsonl");
    let policy = temp.path().join("policy.json");
    std::fs::write(
        &policy,
        json!({"projects":[p],"operations":["get_memory_health"],"audit_path":audit}).to_string(),
    )
    .unwrap();
    let before = std::fs::read(&db).unwrap();
    let report = cli(
        &db,
        &["--policy", policy.to_str().unwrap(), "setup", "--json"],
    );
    assert!(
        report.status.success(),
        "{}",
        String::from_utf8_lossy(&report.stderr)
    );
    let report: Value = serde_json::from_slice(&report.stdout).unwrap();
    assert!(
        report["configuration"]["mcpServers"]["project-memory"]["args"]
            .as_array()
            .unwrap()
            .windows(2)
            .any(|a| a == [json!("--policy"), json!(policy)])
    );
    assert!(!audit.exists());
    let out = cli(
        &db,
        &[
            "--policy",
            policy.to_str().unwrap(),
            "health",
            "--project",
            &p,
            "--max-bytes",
            "4096",
        ],
    );
    assert!(out.status.success());
    let health: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(health["summary"]["record_count"], 1);
    assert!(health["budget"]["used_bytes"].as_u64().unwrap() <= 4096);
    assert_eq!(std::fs::read(&db).unwrap(), before);
    let denied = cli(
        &db,
        &[
            "--policy",
            policy.to_str().unwrap(),
            "health",
            "--project",
            "00000000-0000-4000-8000-000000000001",
        ],
    );
    assert!(!denied.status.success());
    assert!(denied.stdout.is_empty());
}
