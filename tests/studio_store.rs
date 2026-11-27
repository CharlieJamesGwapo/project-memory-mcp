use project_memory_mcp::store::Store;
use serde_json::{Value, json};

fn create(s: &mut Store, name: &str) -> String {
    s.execute(
        "create_project",
        json!({"name":name,"purpose":"Studio test"}),
    )
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
fn note(s: &mut Store, id: &str, text: &str) -> Value {
    s.execute(
        "save_note",
        json!({"project_id":id,"title":"Context","content":text,"author_type":"assistant"}),
    )
    .unwrap()
}
fn decision(s: &mut Store, id: &str, supersedes: Option<&str>) -> Value {
    s.execute("record_decision",json!({"project_id":id,"decision":"Use SQLite","rationale":"Local data","author_type":"user","supersedes":supersedes})).unwrap()
}

#[test]
fn readonly_open_does_not_create_missing_files_or_directories() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing parent/memory.sqlite");
    assert!(Store::open_read_only(&missing).is_err());
    assert!(!missing.parent().unwrap().exists());
}

#[test]
fn readonly_refuses_foreign_database_without_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("foreign.sqlite");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE inventory(id INTEGER PRIMARY KEY)")
        .unwrap();
    drop(db);
    let before = std::fs::read(&path).unwrap();
    let permissions = std::fs::metadata(&path).unwrap().permissions();
    assert!(Store::open_read_only(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode(),
            permissions.mode()
        );
    }
    #[cfg(not(unix))]
    let _ = permissions;
}

#[test]
fn readonly_can_query_but_never_save_or_delete_memories() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("memory.sqlite");
    let mut writer = Store::open(&path).unwrap();
    let id = create(&mut writer, "Atlas");
    note(&mut writer, &id, "Original");
    drop(writer);
    let before = std::fs::read(&path).unwrap();
    let mut reader = Store::open_read_only(&path).unwrap();
    assert_eq!(
        reader
            .execute("get_project_context", json!({"project_id":id}))
            .unwrap()["history"]["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(reader.execute("save_note",json!({"project_id":id,"title":"Forbidden","content":"No write","author_type":"assistant"})).is_err());
    assert!(
        reader
            .execute("delete_project", json!({"project_id":id,"confirm":true}))
            .is_err()
    );
    drop(reader);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let mut reader = Store::open_read_only(&path).unwrap();
    assert_eq!(
        reader.execute("list_projects", json!({})).unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn studio_summaries_count_records_and_active_decisions_exactly() {
    let mut s = Store::memory().unwrap();
    let a = create(&mut s, "Atlas");
    let b = create(&mut s, "Atlas");
    note(&mut s, &a, "One");
    note(&mut s, &a, "Two");
    let old = decision(&mut s, &a, None);
    decision(&mut s, &a, old["id"].as_str());
    let h=s.execute("save_handoff",json!({"project_id":a,"summary":"Ready","completed":[],"unfinished":[],"next_actions":["Review"],"blockers":[],"verification":[],"author_type":"assistant"})).unwrap();
    let page = s.execute("studio_list_projects", json!({})).unwrap();
    let projects = page["items"].as_array().unwrap();
    let summary = projects.iter().find(|v| v["id"] == a).unwrap();
    assert_eq!(summary["record_count"], 5);
    assert_eq!(summary["note_count"], 2);
    assert_eq!(summary["decision_count"], 2);
    assert_eq!(summary["active_decision_count"], 1);
    assert_eq!(summary["handoff_count"], 1);
    assert_eq!(summary["last_activity"], h["created_at"]);
    assert_eq!(
        projects.iter().find(|v| v["id"] == b).unwrap()["record_count"],
        0
    );
}

#[test]
fn readonly_refresh_sees_writes_from_the_existing_mcp_store() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("shared.sqlite");
    let mut writer = Store::open(&path).unwrap();
    let id = create(&mut writer, "Live");
    let mut reader = Store::open_read_only(&path).unwrap();
    assert_eq!(
        reader.execute("studio_list_projects", json!({})).unwrap()["items"][0]["record_count"],
        0
    );
    note(&mut writer, &id, "Written after Studio opened");
    assert_eq!(
        reader.execute("studio_list_projects", json!({})).unwrap()["items"][0]["record_count"],
        1
    );
}

#[test]
fn filtered_records_paginate_matching_kind_without_crossing_projects() {
    let mut s = Store::memory().unwrap();
    let a = create(&mut s, "A");
    let b = create(&mut s, "B");
    let mut ids = vec![];
    for i in 0..5 {
        ids.push(note(&mut s, &a, &format!("note {i}"))["id"].clone());
        decision(&mut s, &a, None);
    }
    note(&mut s, &b, "Foreign");
    let mut page = s
        .execute(
            "studio_list_records",
            json!({"project_id":a,"kind":"note","limit":2}),
        )
        .unwrap();
    let cursor = page["next_cursor"].clone();
    let mut found = vec![];
    loop {
        for value in page["items"].as_array().unwrap() {
            assert_eq!(value["kind"], "note");
            found.push(value["id"].clone());
        }
        if page["next_cursor"].is_null() {
            break;
        }
        page = s
            .execute(
                "studio_list_records",
                json!({"project_id":a,"kind":"note","limit":2,"cursor":page["next_cursor"]}),
            )
            .unwrap();
    }
    ids.reverse();
    assert_eq!(found, ids);
    assert_eq!(
        s.execute(
            "studio_list_records",
            json!({"project_id":b,"kind":"note","cursor":cursor})
        )
        .unwrap_err()
        .code(),
        "INVALID_CURSOR"
    );
    assert_eq!(
        s.execute(
            "studio_list_records",
            json!({"project_id":a,"kind":"decision","cursor":cursor})
        )
        .unwrap_err()
        .code(),
        "INVALID_CURSOR"
    );
    assert_eq!(
        s.execute(
            "studio_list_records",
            json!({"project_id":a,"kind":"wrong"})
        )
        .unwrap_err()
        .code(),
        "VALIDATION_ERROR"
    );
}

#[test]
fn filtered_active_decision_pages_keep_their_original_snapshot() {
    let mut s = Store::memory().unwrap();
    let id = create(&mut s, "Snapshot");
    let old = decision(&mut s, &id, None);
    let first = decision(&mut s, &id, None);
    let page = s
        .execute(
            "studio_list_records",
            json!({"project_id":id,"kind":"decision","include_superseded":false,"limit":1}),
        )
        .unwrap();
    assert_eq!(page["items"][0]["id"], first["id"]);
    decision(&mut s, &id, old["id"].as_str());
    let next=s.execute("studio_list_records",json!({"project_id":id,"kind":"decision","include_superseded":false,"cursor":page["next_cursor"]})).unwrap();
    assert_eq!(next["items"][0]["id"], old["id"]);
    assert!(next["items"][0]["superseded_by"].is_null());
    assert_eq!(
        s.execute(
            "studio_list_records",
            json!({"project_id":id,"kind":"decision"})
        )
        .unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}
