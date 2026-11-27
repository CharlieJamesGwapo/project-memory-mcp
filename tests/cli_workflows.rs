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

fn parse(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn init_exits_without_handshake_and_is_idempotent_with_existing_content() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("new folder/memory.sqlite");
    let first = parse(&cli(&path, &["init", "--json"]));
    assert_eq!(first["status"], "ready");
    assert_eq!(first["diagnostics"]["schema_version"], 1);
    let mut store = Store::open(&path).unwrap();
    store
        .execute(
            "create_project",
            json!({"name":"Keep me","purpose":"Synthetic CLI test"}),
        )
        .unwrap();
    drop(store);
    let before = std::fs::read(&path).unwrap();
    let second = parse(&cli(&path, &["init", "--json"]));
    assert_eq!(second["diagnostics"]["project_count"], 1);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn init_readonly_and_inspection_of_missing_archive_never_create_it() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("missing/memory.sqlite");
    let id = "00000000-0000-4000-8000-000000000001";
    for args in [
        vec!["--read-only", "init"],
        vec!["projects"],
        vec!["search", "--project", id, "needle"],
        vec!["record", "--project", id, "--record", id],
        vec!["context", "--project", id],
    ] {
        let out = cli(&path, &args);
        assert!(!out.status.success(), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert!(!path.parent().unwrap().exists(), "{args:?}");
    }
}

#[test]
fn init_refuses_foreign_and_newer_archives_without_byte_changes() {
    let temp = tempfile::tempdir().unwrap();
    for (name, sql) in [
        (
            "foreign",
            "PRAGMA application_id=99; CREATE TABLE existing(value TEXT);",
        ),
        (
            "newer",
            "PRAGMA application_id=1347240784; PRAGMA user_version=2;",
        ),
    ] {
        let path = temp.path().join(format!("{name}.sqlite"));
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(sql).unwrap();
        drop(conn);
        let before = std::fs::read(&path).unwrap();
        assert!(!cli(&path, &["init"]).status.success());
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}

#[test]
fn read_commands_use_uuid_scoping_pagination_and_complete_original_records() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("memory.sqlite");
    let mut store = Store::open(&path).unwrap();
    let a = store
        .execute(
            "create_project",
            json!({"name":"Same name","purpose":"First"}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let b = store
        .execute(
            "create_project",
            json!({"name":"Same name","purpose":"Second"}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let content = format!(
        "Full original evidence: {} پایان ``` injected fence",
        "unicode界".repeat(100)
    );
    let note = store.execute("save_note", json!({"project_id":a,"title":"Windows authentication","content":content,"author_type":"user","sources":["synthetic.rs:12"]})).unwrap();
    let id = note["id"].as_str().unwrap();
    let other = store.execute("save_note", json!({"project_id":b,"title":"Other archive context","content":"Must not leak","author_type":"assistant"})).unwrap();
    drop(store);
    let before = std::fs::read(&path).unwrap();
    let page = parse(&cli(&path, &["projects", "--limit", "1"]));
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    let cursor = page["next_cursor"].as_str().unwrap();
    let next = parse(&cli(
        &path,
        &["projects", "--limit", "1", "--cursor", cursor],
    ));
    assert_ne!(page["items"][0]["id"], next["items"][0]["id"]);
    let search = parse(&cli(&path, &["search", "--project", &a, "authentication"]));
    assert_eq!(search["items"][0]["id"], id);
    let full = parse(&cli(&path, &["record", "--project", &a, "--record", id]));
    assert_eq!(full["content"], content);
    assert_eq!(full["sources"], json!(["synthetic.rs:12"]));
    let bundle = parse(&cli(
        &path,
        &["context", "--project", &a, "--record", id, "--json"],
    ));
    assert_eq!(bundle["records"][0]["content"], content);
    assert!(bundle["budget"]["used_bytes"].as_u64().unwrap() <= 16000);
    let markdown = cli(&path, &["context", "--project", &a, "--record", id]);
    assert!(markdown.status.success());
    let markdown = String::from_utf8(markdown.stdout).unwrap();
    assert!(markdown.contains("# Project memory context"));
    assert!(markdown.contains("untrusted"));
    assert!(markdown.contains("canonical MCP result"));
    assert!(markdown.contains(&serde_json::to_string(&content).unwrap()));
    let bad = cli(
        &path,
        &[
            "record",
            "--project",
            &a,
            "--record",
            other["id"].as_str().unwrap(),
        ],
    );
    assert!(!bad.status.success());
    assert!(bad.stdout.is_empty());
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn context_terms_flag_uses_ranked_retrieval_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("memory.sqlite");
    let mut store = Store::open(&path).unwrap();
    let p = store
        .execute(
            "create_project",
            json!({"name":"Terms","purpose":"CLI coverage"}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    store.execute("save_note", json!({"project_id":p,"title":"Windows","content":"Authentication behavior","author_type":"user"})).unwrap();
    drop(store);
    let value = parse(&cli(
        &path,
        &[
            "context",
            "--project",
            &p,
            "--query",
            "windows authentication",
            "--query-mode",
            "terms",
            "--json",
        ],
    ));
    assert_eq!(value["selection"]["mode"], "terms");
    assert_eq!(
        value["records"][0]["matched_terms"],
        json!(["windows", "authentication"])
    );
}
