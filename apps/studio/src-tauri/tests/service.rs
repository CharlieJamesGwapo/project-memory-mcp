use project_memory_mcp::store::Store;
use project_memory_studio_lib::service::{StudioSession, validate_markdown, write_markdown};
use serde_json::json;
use std::path::Path;

fn database(path: &Path, name: &str) -> String {
    let mut s = Store::open(path).unwrap();
    s.execute(
        "create_project",
        json!({"name":name,"purpose":"Native service integration"}),
    )
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}

#[test]
fn bridge_reads_review_history_health_and_policy_without_allowing_writes() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("reviews.sqlite");
    let project = database(&path, "Evidence views");
    let mut writer = Store::open(&path).unwrap();
    let mut args = json!({"project_id":project,"repository":"synthetic","base_commit":"a".repeat(40),"head_commit":"b".repeat(40),"summary":"Original review","checks":[],"findings":[],"author_type":"user","sources":["fixture.rs:12"]});
    let original = writer.execute("save_review", args.clone()).unwrap();
    args["summary"] = json!("Revised review");
    args["previous_review_id"] = original["id"].clone();
    let revised = writer.execute("save_review", args).unwrap();
    drop(writer);
    let before = std::fs::read(&path).unwrap();
    let mut session = StudioSession::new(None);
    let id = session.open_database(&path).unwrap().session_id.unwrap();
    let history = session
        .query(
            &id,
            "get_review_history",
            json!({"project_id":project,"current_head":"c".repeat(40)}),
        )
        .unwrap();
    assert_eq!(history["items"][1]["review"]["revised_by"], revised["id"]);
    assert_eq!(history["items"][0]["freshness"], "stale");
    let health = session
        .query(
            &id,
            "get_memory_health",
            json!({"project_id":project,"max_bytes":4096}),
        )
        .unwrap();
    assert_eq!(health["summary"]["record_count"], 2);
    assert_eq!(health["summary"]["current_review_count"], 1);
    let policy = session.query(&id, "get_access_policy", json!({})).unwrap();
    assert_eq!(policy["read_only"], true);
    assert_eq!(policy["allow_delete"], false);
    assert_eq!(
        session
            .query(&id, "save_review", json!({}))
            .unwrap_err()
            .code,
        "OPERATION_NOT_ALLOWED"
    );
    assert_eq!(
        session
            .query(
                "old-session",
                "get_memory_health",
                json!({"project_id":project})
            )
            .unwrap_err()
            .code,
        "STALE_SESSION"
    );
    drop(session);
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn bootstrap_missing_database_does_not_create_anything() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("missing/db.sqlite");
    let mut session = StudioSession::new(Some(path.clone()));
    let info = session.bootstrap();
    assert_eq!(info.status, "missing");
    assert!(info.session_id.is_none());
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn native_queries_reject_writes_and_stale_sessions() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("a.sqlite");
    let b = temp.path().join("b.sqlite");
    let project_a = database(&a, "A");
    database(&b, "B");
    let mut session = StudioSession::new(None);
    let first = session.open_database(&a).unwrap();
    let id = first.session_id.unwrap();
    assert_eq!(
        session
            .query(&id, "studio_list_projects", json!({}))
            .unwrap()["items"][0]["id"],
        project_a
    );
    assert_eq!(
        session
            .query(
                &id,
                "delete_project",
                json!({"project_id":project_a,"confirm":true})
            )
            .unwrap_err()
            .code,
        "OPERATION_NOT_ALLOWED"
    );
    session.open_database(&b).unwrap();
    assert_eq!(
        session
            .query(&id, "list_projects", json!({}))
            .unwrap_err()
            .code,
        "STALE_SESSION"
    );
}

#[test]
fn bridge_accepts_full_budgeted_context_and_unified_search() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("memory.sqlite");
    let project_id = database(&path, "Toolkit");
    let content = format!("{} END OF COMPLETE NOTE", "café 日本語 ".repeat(70));
    let mut writer = Store::open(&path).unwrap();
    writer.execute("save_note",json!({"project_id":project_id,"title":"Evidence","content":content,"author_type":"user"})).unwrap();
    drop(writer);
    let mut session = StudioSession::new(None);
    let id = session.open_database(&path).unwrap().session_id.unwrap();
    let found = session
        .query(
            &id,
            "search_memory",
            json!({"project_id":project_id,"query":"END OF COMPLETE NOTE"}),
        )
        .unwrap();
    assert_eq!(found["items"].as_array().unwrap().len(), 1);
    let bundle = session
        .query(
            &id,
            "get_context_bundle",
            json!({"project_id":project_id,"query":"END OF COMPLETE NOTE","max_bytes":16000}),
        )
        .unwrap();
    assert_eq!(bundle["records"][0]["content"], content);
    assert!(bundle["budget"]["used_bytes"].as_u64().unwrap() <= 16000);
    assert_eq!(bundle["selection"]["selection_complete"], true);
}

#[test]
fn failed_file_selection_preserves_the_previous_connection() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("a.sqlite");
    let foreign = temp.path().join("foreign.sqlite");
    let project_id = database(&a, "A");
    let db = rusqlite::Connection::open(&foreign).unwrap();
    db.execute_batch("CREATE TABLE unrelated(id INTEGER)")
        .unwrap();
    drop(db);
    let mut session = StudioSession::new(None);
    let before = session.open_database(&a).unwrap();
    assert!(session.open_database(&foreign).is_err());
    let after = session.bootstrap();
    assert_eq!(before.session_id, after.session_id);
    assert_eq!(before.path, after.path);
    assert_eq!(
        session
            .query(
                after.session_id.as_ref().unwrap(),
                "list_projects",
                json!({})
            )
            .unwrap()["items"][0]["id"],
        project_id
    );
}

#[test]
fn context_size_bounds_are_checked_before_clipboard_or_file_actions() {
    assert!(validate_markdown("").is_err());
    assert!(validate_markdown(" \n").is_err());
    assert!(validate_markdown(&"x".repeat(2 * 1024 * 1024 + 1)).is_err());
    assert!(validate_markdown("# Context\nSaved evidence").is_ok());
}

#[test]
fn export_is_exact_and_refuses_database_or_non_markdown_destinations() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("context.md");
    write_markdown(&target, "# Context\n日本語", None).unwrap();
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "# Context\n日本語"
    );
    let db = temp.path().join("memory.md");
    database(&db, "A");
    let before = std::fs::read(&db).unwrap();
    assert!(write_markdown(&db, "Don't replace", Some(&db)).is_err());
    assert!(write_markdown(&db, "Don't replace", None).is_err());
    assert_eq!(std::fs::read(&db).unwrap(), before);
    assert!(write_markdown(&temp.path().join("file.txt"), "Text", None).is_err());
}
