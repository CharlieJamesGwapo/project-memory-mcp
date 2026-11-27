use project_memory_mcp::{security::AccessPolicy, store::Store};
use rmcp::model::CallToolResult;
use serde_json::{Value, json};

fn project(store: &mut Store) -> String {
    store
        .execute(
            "create_project",
            json!({"name":"Synthetic health","purpose":"Evidence-only tests"}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
fn note(store: &mut Store, p: &str, title: &str, content: &str, sources: Value) -> Value {
    store.execute("save_note", json!({"project_id":p,"title":title,"content":content,"author_type":"user","sources":sources})).unwrap()
}
fn review(p: &str) -> Value {
    json!({"project_id":p,"repository":"synthetic/repo","base_commit":"a".repeat(40),"head_commit":"b".repeat(40),"summary":"Synthetic review","checks":[],"findings":[{"id":"f1","severity":"low","title":"Follow up","path":"src/lib.rs","line":1,"evidence":"Synthetic evidence","correction":"Inspect","status":"open"}],"author_type":"assistant","sources":["src/lib.rs:1"]})
}
fn health(store: &mut Store, p: &str) -> Value {
    store
        .execute("get_memory_health", json!({"project_id":p}))
        .expect("health read must be supported")
}
fn codes(item: &Value) -> Vec<&str> {
    item["signals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["code"].as_str().unwrap())
        .collect()
}
fn item<'a>(page: &'a Value, record: &Value) -> &'a Value {
    page["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["record_id"] == record["id"])
        .unwrap()
}

#[test]
fn health_assesses_literal_evidence_without_exposing_saved_text() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store);
    let a = note(&mut store, &p, "Exact", "PRIVATE_LITERAL_NOTE", json!([]));
    let b = note(
        &mut store,
        &p,
        "Exact",
        "PRIVATE_LITERAL_NOTE",
        json!(["synthetic:source"]),
    );
    let c = note(
        &mut store,
        &p,
        "Different",
        "PRIVATE_LITERAL_NOTE",
        json!(["synthetic:source"]),
    );
    let d = note(
        &mut store,
        &p,
        "Exact",
        "private_literal_note",
        json!(["synthetic:source"]),
    );
    let page = health(&mut store, &p);
    assert_eq!(page["summary"]["record_count"], 4);
    assert_eq!(page["summary"]["missing_source_count"], 1);
    assert_eq!(page["records_examined"], 4);
    assert_eq!(
        codes(item(&page, &a)),
        ["missing_sources", "repeated_literal_note_text"]
    );
    assert_eq!(codes(item(&page, &b)), ["repeated_literal_note_text"]);
    assert!(codes(item(&page, &c)).is_empty());
    assert!(codes(item(&page, &d)).is_empty());
    assert!(!page.to_string().contains("PRIVATE_LITERAL_NOTE"));
    assert!(!page.to_string().contains("synthetic:source"));
    assert!(page["notice"].as_str().unwrap().contains("historical"));
    assert!(page["next_cursor"].is_null());
}

#[test]
fn current_revision_statuses_and_supplied_heads_are_the_only_review_signals() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store);
    let first = store.execute("save_review", review(&p)).unwrap();
    let mut next = review(&p);
    next["previous_review_id"] = first["id"].clone();
    next["head_commit"] = json!("c".repeat(40));
    next["findings"][0]["status"] = json!("fixed");
    let second = store.execute("save_review", next).unwrap();
    let page = store
        .execute(
            "get_memory_health",
            json!({"project_id":p,"current_head":"b".repeat(40)}),
        )
        .unwrap();
    assert_eq!(page["summary"]["review_count"], 2);
    assert_eq!(page["summary"]["current_review_count"], 1);
    assert_eq!(page["summary"]["open_finding_count"], 0);
    assert!(codes(item(&page, &first)).is_empty());
    assert_eq!(codes(item(&page, &second)), ["review_head_changed"]);
    assert!(codes(item(&health(&mut store, &p), &second)).is_empty());
    let mut follow = review(&p);
    follow["previous_review_id"] = second["id"].clone();
    follow["findings"][0]["status"] = json!("needs_verification");
    let third = store.execute("save_review", follow).unwrap();
    let page = store
        .execute(
            "get_memory_health",
            json!({"project_id":p,"current_head":"b".repeat(40)}),
        )
        .unwrap();
    assert_eq!(page["summary"]["open_finding_count"], 1);
    assert_eq!(codes(item(&page, &third)), ["review_follow_up"]);
}

#[test]
fn only_latest_unfinished_handoff_and_independent_decision_count_suggest_review() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store);
    let handoff = json!({"project_id":p,"summary":"Synthetic handoff","completed":[],"unfinished":["Remaining"],"next_actions":[],"blockers":[],"verification":[],"author_type":"user","sources":["synthetic"]});
    let old = store.execute("save_handoff", handoff.clone()).unwrap();
    let initial = health(&mut store, &p);
    assert_eq!(
        codes(item(&initial, &old)),
        ["unfinished_handoff_without_next_actions"]
    );
    let mut next = handoff;
    next["next_actions"] = json!(["Inspect remaining work"]);
    let latest = store.execute("save_handoff", next).unwrap();
    let decision = json!({"project_id":p,"decision":"Same compatible choice","rationale":"Synthetic","author_type":"user","sources":["synthetic"]});
    let first = store.execute("record_decision", decision.clone()).unwrap();
    store.execute("record_decision", decision.clone()).unwrap();
    let mut successor = decision;
    successor["supersedes"] = first["id"].clone();
    store.execute("record_decision", successor).unwrap();
    let page = health(&mut store, &p);
    assert_eq!(page["summary"]["active_decision_count"], 2);
    assert_eq!(page["summary"]["latest_handoff_id"], latest["id"]);
    assert!(codes(item(&page, &old)).is_empty());
    assert!(codes(item(&page, &latest)).is_empty());
    assert_eq!(
        page["summary"]["suggestions"][0]["code"],
        "multiple_active_decisions"
    );
    assert!(!page.to_string().contains("conflict"));
}

#[test]
fn snapshot_pages_advance_without_signals_and_exclude_future_revision_and_duplicate_links() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store);
    let q = project(&mut store);
    note(&mut store, &q, "Same", "Original", json!([]));
    let oldest = note(&mut store, &p, "Same", "Original", json!(["synthetic"]));
    let first_review = store.execute("save_review", review(&p)).unwrap();
    note(&mut store, &p, "Newest", "Clean", json!(["synthetic"]));
    let page = store
        .execute("get_memory_health", json!({"project_id":p,"limit":1}))
        .unwrap();
    assert_eq!(page["records_examined"], 1);
    assert!(codes(&page["items"][0]).is_empty());
    let mut revision = review(&p);
    revision["previous_review_id"] = first_review["id"].clone();
    revision["findings"] = json!([]);
    store.execute("save_review", revision).unwrap();
    note(&mut store, &p, "Same", "Original", json!(["synthetic"]));
    let next = store
        .execute(
            "get_memory_health",
            json!({"project_id":p,"limit":1,"cursor":page["next_cursor"]}),
        )
        .unwrap();
    assert_eq!(next["summary"], page["summary"]);
    assert_eq!(next["summary"]["open_finding_count"], 1);
    assert_eq!(next["items"][0]["record_id"], first_review["id"]);
    assert_eq!(codes(&next["items"][0]), ["review_follow_up"]);
    let last = store
        .execute(
            "get_memory_health",
            json!({"project_id":p,"limit":1,"cursor":next["next_cursor"]}),
        )
        .unwrap();
    assert_eq!(last["items"][0]["record_id"], oldest["id"]);
    assert!(codes(&last["items"][0]).is_empty());
    assert!(last["next_cursor"].is_null());
    for args in [
        json!({"project_id":q,"cursor":page["next_cursor"]}),
        json!({"project_id":p,"current_head":"b".repeat(40),"cursor":page["next_cursor"]}),
    ] {
        assert_eq!(
            store.execute("get_memory_health", args).unwrap_err().code(),
            "INVALID_CURSOR"
        );
    }
}

#[test]
fn canonical_budget_preserves_complete_assessments_and_all_rows_are_reachable() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store);
    for _ in 0..15 {
        note(&mut store, &p, "Repeated", "Same", json!([]));
    }
    let mut cursor = Value::Null;
    let mut count = 0;
    loop {
        let page = store
            .execute(
                "get_memory_health",
                json!({"project_id":p,"limit":50,"max_bytes":4096,"cursor":cursor}),
            )
            .unwrap();
        let bytes = serde_json::to_vec(&CallToolResult::structured(page.clone()))
            .unwrap()
            .len();
        assert_eq!(page["budget"]["used_bytes"], bytes);
        assert!(bytes <= 4096);
        assert_eq!(page["summary"]["record_count"], 15);
        let items = page["items"].as_array().unwrap();
        assert!(!items.is_empty());
        assert_eq!(page["records_examined"], items.len());
        for row in items {
            assert_eq!(
                codes(row),
                ["missing_sources", "repeated_literal_note_text"]
            );
        }
        count += items.len();
        cursor = page["next_cursor"].clone();
        assert_eq!(page["truncated"], !cursor.is_null());
        if cursor.is_null() {
            break;
        }
    }
    assert_eq!(count, 15);
}

#[test]
fn strict_inputs_hidden_projects_and_read_only_history_are_preserved() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("synthetic.sqlite");
    let mut store = Store::open(&path).unwrap();
    let p = project(&mut store);
    let q = project(&mut store);
    note(&mut store, &p, "Synthetic", "kept", json!([]));
    for args in [
        json!({"project_id":p,"limit":0}),
        json!({"project_id":p,"limit":51}),
        json!({"project_id":p,"current_head":"main"}),
        json!({"project_id":p,"current_head":"A".repeat(40)}),
        json!({"project_id":p,"max_bytes":4095}),
        json!({"project_id":p,"max_bytes":65537}),
        json!({"project_id":p,"unknown":true}),
    ] {
        assert!(store.execute("get_memory_health", args).is_err());
    }
    let before = store.diagnostics().unwrap();
    drop(store);
    let mut readonly = Store::open_read_only(&path).unwrap();
    let page = health(&mut readonly, &p);
    assert_eq!(page["summary"]["record_count"], 1);
    assert_eq!(
        readonly.diagnostics().unwrap()["record_count"],
        before["record_count"]
    );
    assert_eq!(readonly.diagnostics().unwrap()["schema_version"], 1);
    let policy = AccessPolicy {
        projects: Some(vec![p.clone()]),
        ..AccessPolicy::default()
    };
    readonly.set_policy(policy).unwrap();
    assert_eq!(
        readonly
            .execute("get_memory_health", json!({"project_id":q}))
            .unwrap_err()
            .code(),
        "ACCESS_DENIED"
    );
    assert_eq!(health(&mut readonly, &p)["summary"]["record_count"], 1);
}

#[test]
fn smallest_budget_keeps_all_signals_when_page_length_is_reduced() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store);
    for _ in 0..2 {
        store.execute("record_decision", json!({"project_id":p,"decision":"Compatible choice","rationale":"Synthetic","author_type":"user"})).unwrap();
    }
    let mut input = review(&p);
    input["sources"] = json!([]);
    store.execute("save_review", input.clone()).unwrap();
    let saved = store.execute("save_review", input).unwrap();
    let args = json!({"project_id":p,"current_head":"c".repeat(64),"max_bytes":4096});
    let bounded = store.execute("get_memory_health", args).unwrap();
    assert_eq!(bounded["records_examined"], 1);
    assert_eq!(bounded["truncated"], true);
    assert_eq!(
        codes(item(&bounded, &saved)),
        [
            "missing_sources",
            "repeated_literal_note_text",
            "review_head_changed",
            "review_follow_up"
        ]
    );
    let full = store
        .execute(
            "get_memory_health",
            json!({"project_id":p,"current_head":"c".repeat(64),"max_bytes":65536}),
        )
        .unwrap();
    assert_eq!(
        codes(item(&full, &saved)),
        [
            "missing_sources",
            "repeated_literal_note_text",
            "review_head_changed",
            "review_follow_up"
        ]
    );
    assert_eq!(full["summary"]["current_review_count"], 2);
}

#[test]
fn empty_project_is_a_complete_bounded_report_with_explicit_zero_counts() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store);
    let page = health(&mut store, &p);
    assert_eq!(
        page["summary"],
        json!({"record_count":0,"active_decision_count":0,"review_count":0,"current_review_count":0,"open_finding_count":0,"missing_source_count":0,"latest_handoff_id":null,"suggestions":[]})
    );
    assert_eq!(page["items"], json!([]));
    assert_eq!(page["records_examined"], 0);
    assert_eq!(page["next_cursor"], Value::Null);
    assert_eq!(page["truncated"], false);
    assert_eq!(page["budget"]["max_bytes"], 16000);
    assert_eq!(
        page["budget"]["used_bytes"],
        serde_json::to_vec(&CallToolResult::structured(page.clone()))
            .unwrap()
            .len()
    );
}
