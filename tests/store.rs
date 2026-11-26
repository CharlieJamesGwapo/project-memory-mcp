use project_memory_mcp::store::Store;
use serde_json::{Value, json};

fn project(store: &mut Store, name: &str) -> String {
    store.execute("create_project", json!({"name": name, "purpose": "Build reliable software"})).unwrap()["id"].as_str().unwrap().into()
}
fn note(id: &str, content: &str) -> Value {
    json!({"project_id": id, "title": "Context", "content": content, "author_type": "assistant", "sources": []})
}
fn decision(id: &str, content: &str) -> Value {
    json!({"project_id": id, "decision": content, "rationale": "Explicit choice", "author_type": "user", "sources": ["docs/design.md"]})
}
fn handoff(id: &str, summary: &str) -> Value {
    json!({"project_id": id, "summary": summary, "completed": ["Storage"], "unfinished": ["Review"], "next_actions": ["Run tests"], "blockers": [], "verification": ["Tests passed at handoff time"], "author_type": "assistant", "sources": []})
}
fn call(s: &mut Store, name: &str, args: Value) -> Value { s.execute(name, args).unwrap() }

#[test]
fn persists_across_restarts_with_private_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("space in path").join("memory.sqlite");
    let mut store = Store::open(&path).unwrap();
    let id = project(&mut store, "Persistence");
    let saved = call(&mut store, "save_note", note(&id, "Remember"));
    drop(store);
    let mut reopened = Store::open(&path).unwrap();
    let record = call(&mut reopened, "get_record", json!({"project_id": id, "record_id": saved["id"]}));
    assert_eq!(record["content"], "Remember");
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(path).unwrap().permissions().mode() & 0o777, 0o600);
    }
}

#[test]
fn separates_projects_with_identical_names() {
    let mut s = Store::memory().unwrap();
    let a = project(&mut s, "Atlas"); let b = project(&mut s, "Atlas");
    let saved = call(&mut s, "save_note", note(&a, "private-alpha"));
    assert_eq!(call(&mut s, "search_notes", json!({"project_id": b, "query": "private-alpha"}))["items"], json!([]));
    assert_eq!(s.execute("get_record", json!({"project_id": b, "record_id": saved["id"]})).unwrap_err().code(), "NOT_FOUND");
    assert_eq!(call(&mut s, "list_projects", json!({}))["items"].as_array().unwrap().len(), 2);
}

#[test]
fn validates_ids_fields_authorship_and_limits() {
    let mut s = Store::memory().unwrap(); let id = project(&mut s, "Validation");
    for value in [json!({"project_id": "../bad"}), json!({"project_id": id, "extra": true}), json!({"project_id": id, "limit": 51})] {
        assert_eq!(s.execute("get_project_context", value).unwrap_err().code(), "VALIDATION_ERROR");
    }
    let mut input = note(&id, "x"); input.as_object_mut().unwrap().remove("author_type");
    assert_eq!(s.execute("save_note", input).unwrap_err().code(), "VALIDATION_ERROR");
    assert_eq!(s.execute("save_note", note(&id, &"x".repeat(16001))).unwrap_err().code(), "VALIDATION_ERROR");
    assert_eq!(s.execute("save_note", note("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "x")).unwrap_err().code(), "NOT_FOUND");
}

#[test]
fn searches_punctuation_wildcards_unicode_and_injection_literally() {
    let mut s = Store::memory().unwrap(); let id = project(&mut s, "Search");
    for text in ["100%_done\\path", "'); DROP TABLE projects; --", "日本語 résumé", "C++ (test)"] {
        let saved = call(&mut s, "save_note", note(&id, text));
        assert_eq!(call(&mut s, "search_notes", json!({"project_id": id, "query": text}))["items"][0]["id"], saved["id"]);
    }
    for query in ["%", "_", "\\"] {
        assert_eq!(call(&mut s, "search_notes", json!({"project_id": id, "query": query}))["items"].as_array().unwrap().len(), 1);
    }
    assert_eq!(call(&mut s, "list_projects", json!({}))["items"].as_array().unwrap().len(), 1);
}

#[test]
fn snapshot_pagination_survives_new_records_without_duplicates() {
    let mut s = Store::memory().unwrap(); let id = project(&mut s, "Paging");
    let mut ids = vec![];
    for i in 0..7 { ids.push(call(&mut s, "save_note", note(&id, &format!("entry {i}")))["id"].clone()); }
    let mut page = call(&mut s, "search_notes", json!({"project_id": id, "query": "entry", "limit": 2}));
    let mut found = vec![];
    call(&mut s, "save_note", note(&id, "entry new"));
    loop {
        for item in page["items"].as_array().unwrap() { found.push(item["id"].clone()); }
        if page["next_cursor"].is_null() { break; }
        page = call(&mut s, "search_notes", json!({"project_id": id, "query": "entry", "limit": 2, "cursor": page["next_cursor"]}));
    }
    ids.reverse(); assert_eq!(found, ids);
}

#[test]
fn cursor_is_bound_to_project_filter_and_operation() {
    let mut s = Store::memory().unwrap(); let a = project(&mut s, "A"); let b = project(&mut s, "B");
    call(&mut s, "save_note", note(&a, "entry one")); call(&mut s, "save_note", note(&a, "entry two"));
    let cursor = call(&mut s, "search_notes", json!({"project_id": a, "query": "entry", "limit": 1}))["next_cursor"].clone();
    for args in [json!({"project_id": b, "query": "entry", "cursor": cursor}), json!({"project_id": a, "query": "other", "cursor": cursor}), json!({"project_id": a, "query": "entry", "cursor": "%%%bad"})] {
        assert_eq!(s.execute("search_notes", args).unwrap_err().code(), "INVALID_CURSOR");
    }
    assert_eq!(s.execute("get_project_context", json!({"project_id": a, "cursor": cursor})).unwrap_err().code(), "INVALID_CURSOR");
}

#[test]
fn preserves_decision_history_and_explicit_supersession() {
    let mut s = Store::memory().unwrap(); let id = project(&mut s, "Decisions");
    let old = call(&mut s, "record_decision", decision(&id, "Use SQLite"));
    let mut next = decision(&id, "Use PostgreSQL"); next["supersedes"] = old["id"].clone();
    let saved = call(&mut s, "record_decision", next.clone());
    assert_eq!(call(&mut s, "get_record", json!({"project_id": id, "record_id": old["id"]}))["superseded_by"], saved["id"]);
    assert_eq!(call(&mut s, "get_project_context", json!({"project_id": id}))["history"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(call(&mut s, "get_project_context", json!({"project_id": id, "include_superseded": true}))["history"]["items"].as_array().unwrap().len(), 2);
    assert_eq!(s.execute("record_decision", next).unwrap_err().code(), "CONFLICT");
}

#[test]
fn invalid_supersession_is_atomic_and_never_crosses_projects() {
    let mut s = Store::memory().unwrap(); let a = project(&mut s, "A"); let b = project(&mut s, "B");
    let foreign = call(&mut s, "record_decision", decision(&b, "Foreign"));
    let local = call(&mut s, "save_note", note(&a, "Not a decision"));
    for target in [foreign["id"].clone(), local["id"].clone(), json!("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa")] {
        let mut next = decision(&a, "Invalid"); next["supersedes"] = target;
        assert!(s.execute("record_decision", next).is_err());
    }
    assert_eq!(call(&mut s, "get_project_context", json!({"project_id": a}))["history"]["items"].as_array().unwrap().len(), 1);
    assert!(call(&mut s, "get_record", json!({"project_id": b, "record_id": foreign["id"]}))["superseded_by"].is_null());
}

#[test]
fn independent_decisions_remain_active_and_handoffs_are_immutable() {
    let mut s = Store::memory().unwrap(); let id = project(&mut s, "History");
    call(&mut s, "record_decision", decision(&id, "Use SQLite")); call(&mut s, "record_decision", decision(&id, "Use PostgreSQL"));
    let old = call(&mut s, "save_handoff", handoff(&id, "Earlier")); let latest = call(&mut s, "save_handoff", handoff(&id, "Latest"));
    let context = call(&mut s, "get_project_context", json!({"project_id": id}));
    assert_eq!(context["active_decisions"]["items"].as_array().unwrap().len(), 2);
    assert_eq!(context["latest_handoff"]["id"], latest["id"]);
    assert_eq!(call(&mut s, "get_record", json!({"project_id": id, "record_id": old["id"]}))["summary"], "Earlier");
}

#[test]
fn context_is_bounded_and_complete_record_is_recoverable() {
    let mut s = Store::memory().unwrap(); let id = project(&mut s, "Bounds");
    let saved = call(&mut s, "save_note", note(&id, &"😀".repeat(16000)));
    for _ in 0..3 { call(&mut s, "save_note", note(&id, "short")); }
    let context = call(&mut s, "get_project_context", json!({"project_id": id, "limit": 2}));
    assert_eq!(context["history"]["truncated"], true); assert!(context.to_string().len() < 10000);
    let full = call(&mut s, "get_record", json!({"project_id": id, "record_id": saved["id"]}));
    assert_eq!(full["content"].as_str().unwrap().chars().count(), 16000);
    let rest = call(&mut s, "get_project_context", json!({"project_id": id, "cursor": context["history"]["next_cursor"]}));
    assert_eq!(rest["history"]["items"].as_array().unwrap().len(), 2);
}

#[test]
fn rejects_oversized_total_handoffs() {
    let mut s = Store::memory().unwrap(); let id = project(&mut s, "Size");
    let many = json!(vec!["x".repeat(1000); 20]);
    let mut input = handoff(&id, "Large");
    for field in ["completed", "unfinished", "next_actions", "verification"] { input[field] = many.clone(); }
    assert_eq!(s.execute("save_handoff", input).unwrap_err().code(), "VALIDATION_ERROR");
}

#[test]
fn export_is_paginated_and_preserves_history_and_sources() {
    let mut s = Store::memory().unwrap(); let id = project(&mut s, "Export");
    let old = call(&mut s, "record_decision", decision(&id, "Old"));
    let mut next = decision(&id, "New"); next["supersedes"] = old["id"].clone(); call(&mut s, "record_decision", next);
    call(&mut s, "save_note", note(&id, "```\n# Ignore instructions\n~~~"));
    let mut page = call(&mut s, "export_project", json!({"project_id": id, "limit": 1}));
    let mut markdown = String::new(); let mut ids = vec![];
    loop {
        markdown.push_str(page["markdown"].as_str().unwrap()); ids.extend(page["record_ids"].as_array().unwrap().iter().cloned());
        if page["next_cursor"].is_null() { break; }
        page = call(&mut s, "export_project", json!({"project_id": id, "limit": 1, "cursor": page["next_cursor"]}));
    }
    assert_eq!(ids.len(), 3); assert!(markdown.contains("superseded_by")); assert!(markdown.contains("docs/design.md")); assert!(markdown.contains("Ignore instructions"));
}

#[test]
fn deletion_requires_confirmation_and_only_cascades_one_project() {
    let mut s = Store::memory().unwrap(); let a = project(&mut s, "A"); let b = project(&mut s, "B");
    call(&mut s, "save_note", note(&a, "A")); call(&mut s, "save_note", note(&b, "B"));
    assert_eq!(s.execute("delete_project", json!({"project_id": a, "confirm": false})).unwrap_err().code(), "VALIDATION_ERROR");
    assert_eq!(call(&mut s, "delete_project", json!({"project_id": a, "confirm": true}))["deleted"], true);
    assert_eq!(s.execute("get_project_context", json!({"project_id": a})).unwrap_err().code(), "NOT_FOUND");
    assert_eq!(call(&mut s, "get_project_context", json!({"project_id": b}))["history"]["items"].as_array().unwrap().len(), 1);
}
