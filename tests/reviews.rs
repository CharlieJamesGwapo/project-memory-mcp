use project_memory_mcp::store::Store;
use rmcp::model::CallToolResult;
use serde_json::{Value, json};

fn project(store: &mut Store, name: &str) -> String {
    store
        .execute(
            "create_project",
            json!({"name":name,"purpose":"Synthetic review tests"}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}

fn review(project: &str) -> Value {
    json!({
        "project_id":project,"repository":"example/local-repo",
        "base_commit":"a".repeat(40),"head_commit":"b".repeat(40),
        "summary":"Authorization boundary review",
        "checks":[{"name":"cargo test","outcome":"passed","result":"12 synthetic checks passed"}],
        "findings":[{"id":"access-1","severity":"high","title":"Missing project scope",
            "path":"src/access.rs","line":12,"evidence":"A caller can select another project",
            "correction":"Apply scope before querying","status":"open"}],
        "author_type":"assistant","sources":["src/access.rs:12"]
    })
}

#[test]
fn structured_review_is_an_immutable_complete_schema_one_note() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store, "Reviews");
    let input = review(&p);
    let saved = store.execute("save_review", input.clone()).unwrap();
    assert_eq!(saved["kind"], "note");
    assert_eq!(saved["author_type"], "assistant");
    assert_eq!(saved["sources"], input["sources"]);
    assert_eq!(saved["review"]["format"], "project-memory-review-v1");
    assert_eq!(saved["review"]["findings"], input["findings"]);
    assert_eq!(saved["review"]["checks"], input["checks"]);
    assert!(
        saved["content"]
            .as_str()
            .unwrap()
            .contains("Apply scope before querying")
    );
    assert_eq!(store.diagnostics().unwrap()["schema_version"], 1);
    let fetched = store
        .execute(
            "get_record",
            json!({"project_id":p,"record_id":saved["id"]}),
        )
        .unwrap();
    assert_eq!(saved, fetched);
    let searched = store
        .execute(
            "search_memory",
            json!({"project_id":p,"query":"Missing project scope"}),
        )
        .unwrap();
    assert_eq!(searched["items"][0]["id"], saved["id"]);
    let exported = store
        .execute("export_project", json!({"project_id":p}))
        .unwrap();
    assert!(
        exported["markdown"]
            .as_str()
            .unwrap()
            .contains("project-memory-review-v1")
    );
}

#[test]
fn revisions_preserve_history_reject_forks_and_report_stale_evidence() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store, "Reviews");
    let first = store.execute("save_review", review(&p)).unwrap();
    let mut updated = review(&p);
    updated["previous_review_id"] = first["id"].clone();
    updated["head_commit"] = json!("c".repeat(40));
    updated["findings"][0]["status"] = json!("fixed");
    let second = store.execute("save_review", updated.clone()).unwrap();
    assert!(
        store.execute("save_review", updated).is_err(),
        "revision fork must fail"
    );
    let history = store
        .execute(
            "get_review_history",
            json!({"project_id":p,"current_head":"c".repeat(40)}),
        )
        .unwrap();
    assert_eq!(history["items"].as_array().unwrap().len(), 2);
    assert_eq!(history["items"][0]["id"], second["id"]);
    assert_eq!(history["items"][0]["freshness"], "current");
    assert_eq!(history["items"][1]["freshness"], "stale");
    assert_eq!(
        history["items"][1]["review"]["findings"][0]["status"],
        "open"
    );
    assert_eq!(history["items"][1]["review"]["revised_by"], second["id"]);
    let unknown = store
        .execute("get_review_history", json!({"project_id":p}))
        .unwrap();
    assert_eq!(unknown["items"][0]["freshness"], "unknown");
}

#[test]
fn revisions_require_a_review_in_the_same_project_and_repository() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store, "A");
    let q = project(&mut store, "B");
    let saved = store.execute("save_review", review(&p)).unwrap();
    let ordinary = store.execute("save_note",json!({"project_id":p,"title":"Ordinary","content":"No review metadata","author_type":"user"})).unwrap();
    for (project, previous, repository) in [
        (&q, saved["id"].clone(), "example/local-repo"),
        (&p, ordinary["id"].clone(), "example/local-repo"),
        (&p, saved["id"].clone(), "different/repo"),
    ] {
        let mut input = review(project);
        input["previous_review_id"] = previous;
        input["repository"] = json!(repository);
        assert!(store.execute("save_review", input).is_err());
    }
    assert_eq!(
        store
            .execute("get_review_history", json!({"project_id":q}))
            .unwrap()["items"],
        json!([])
    );
}

#[test]
fn history_cursor_is_scoped_and_excludes_future_inserts_and_revision_links() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store, "A");
    let q = project(&mut store, "B");
    let first = store.execute("save_review", review(&p)).unwrap();
    let second = store.execute("save_review", review(&p)).unwrap();
    let page = store
        .execute("get_review_history", json!({"project_id":p,"limit":1}))
        .unwrap();
    assert_eq!(page["items"][0]["id"], second["id"]);
    let cursor = page["next_cursor"].clone();
    let mut next_review = review(&p);
    next_review["previous_review_id"] = first["id"].clone();
    store.execute("save_review", next_review).unwrap();
    let older = store
        .execute(
            "get_review_history",
            json!({"project_id":p,"limit":1,"cursor":cursor}),
        )
        .unwrap();
    assert_eq!(older["items"][0]["id"], first["id"]);
    assert!(older["items"][0]["review"]["revised_by"].is_null());
    assert!(older["next_cursor"].is_null());
    assert!(
        store
            .execute(
                "get_review_history",
                json!({"project_id":q,"cursor":cursor})
            )
            .is_err()
    );
    assert!(
        store
            .execute(
                "get_review_history",
                json!({"project_id":p,"cursor":cursor,"current_head":"b".repeat(40)})
            )
            .is_err()
    );
}

#[test]
fn history_counts_complete_canonical_result_bytes_and_never_truncates_a_review() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store, "Budgets");
    let mut input = review(&p);
    input["findings"][0]["evidence"] = json!("Evidence with quotes \" and Unicode界 ".repeat(25));
    let saved = store.execute("save_review", input).unwrap();
    store.execute("save_review", review(&p)).unwrap();
    let page = store
        .execute(
            "get_review_history",
            json!({"project_id":p,"max_bytes":8192}),
        )
        .unwrap();
    let bytes = serde_json::to_vec(&CallToolResult::structured(page.clone()))
        .unwrap()
        .len();
    assert!(bytes <= 8192);
    assert_eq!(page["budget"]["used_bytes"], bytes);
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert!(!page["next_cursor"].is_null());
    let cursor = page["next_cursor"].clone();
    assert!(
        store
            .execute(
                "get_review_history",
                json!({"project_id":p,"cursor":cursor,"max_bytes":4096})
            )
            .is_err()
    );
    let full = store
        .execute(
            "get_review_history",
            json!({"project_id":p,"cursor":cursor,"max_bytes":65536}),
        )
        .unwrap();
    assert_eq!(
        full["items"][0]["review"]["findings"],
        saved["review"]["findings"]
    );
}

#[test]
fn rejects_invalid_commits_paths_duplicates_unknown_fields_and_oversized_inputs() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store, "Validation");
    for field in ["base_commit", "head_commit"] {
        for bad in ["main", "abc", &"A".repeat(40), &"g".repeat(40)] {
            let mut input = review(&p);
            input[field] = json!(bad);
            assert!(store.execute("save_review", input).is_err());
        }
    }
    for path in [
        "../secret",
        "/absolute",
        "C:\\secret",
        "src/../secret",
        "src\\file.rs",
        "src//file.rs",
        "src/./file.rs",
    ] {
        let mut input = review(&p);
        input["findings"][0]["path"] = json!(path);
        assert!(store.execute("save_review", input).is_err(), "{path}");
    }
    for mutate in [
        |v: &mut Value| v["unexpected"] = json!(true),
        |v: &mut Value| v["findings"][0]["line"] = json!(0),
        |v: &mut Value| v["findings"][0]["status"] = json!("approved"),
        |v: &mut Value| v["findings"][0]["evidence"] = json!("x".repeat(2001)),
        |v: &mut Value| {
            let duplicate = v["findings"][0].clone();
            v["findings"].as_array_mut().unwrap().push(duplicate);
        },
    ] {
        let mut input = review(&p);
        mutate(&mut input);
        assert!(store.execute("save_review", input).is_err());
    }
    for args in [
        json!({"project_id":p,"limit":0}),
        json!({"project_id":p,"limit":21}),
        json!({"project_id":p,"current_head":"main"}),
        json!({"project_id":p,"max_bytes":65537}),
    ] {
        assert!(store.execute("get_review_history", args).is_err());
    }
    assert_eq!(
        store
            .execute("get_review_history", json!({"project_id":p}))
            .unwrap()["items"],
        json!([])
    );
}

#[test]
fn read_only_archives_can_read_reviews_but_cannot_save_them() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("memory.sqlite");
    let mut store = Store::open(&path).unwrap();
    let p = project(&mut store, "Readonly");
    store.execute("save_review", review(&p)).unwrap();
    drop(store);
    let mut readonly = Store::open_read_only(&path).unwrap();
    assert_eq!(
        readonly
            .execute("get_review_history", json!({"project_id":p}))
            .unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        readonly
            .execute("save_review", review(&p))
            .unwrap_err()
            .code(),
        "READ_ONLY"
    );
}

#[test]
fn full_sha256_commits_and_clean_reviews_are_supported() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store, "SHA256");
    let mut input = review(&p);
    input["base_commit"] = json!("a".repeat(64));
    input["head_commit"] = json!("b".repeat(64));
    input["findings"] = json!([]);
    input["checks"] = json!([{"name":"interactive desktop check","outcome":"not_run","result":"Host does not provide native UI access"}]);
    let saved = store.execute("save_review", input).unwrap();
    let result = store
        .execute(
            "get_review_history",
            json!({"project_id":p,"current_head":"b".repeat(64)}),
        )
        .unwrap();
    assert_eq!(result["items"][0]["id"], saved["id"]);
    assert_eq!(result["items"][0]["freshness"], "current");
    assert_eq!(result["items"][0]["review"]["findings"], json!([]));
    assert_eq!(
        result["items"][0]["review"]["checks"][0]["outcome"],
        "not_run"
    );
}

#[test]
fn simultaneous_successors_cannot_fork_a_review_chain() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("memory.sqlite");
    let mut store = Store::open(&path).unwrap();
    let p = project(&mut store, "Concurrent review");
    let first = store.execute("save_review", review(&p)).unwrap();
    drop(store);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let path = path.clone();
        let p = p.clone();
        let previous = first["id"].clone();
        let barrier = barrier.clone();
        workers.push(std::thread::spawn(move || {
            let mut store = Store::open(&path).unwrap();
            let mut input = review(&p);
            input["previous_review_id"] = previous;
            barrier.wait();
            store.execute("save_review", input).is_ok()
        }));
    }
    let successes = workers
        .into_iter()
        .filter_map(|worker| worker.join().unwrap().then_some(()))
        .count();
    assert_eq!(successes, 1);
    let mut store = Store::open(&path).unwrap();
    let result = store
        .execute("get_review_history", json!({"project_id":p}))
        .unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 2);
}

#[test]
fn combined_legal_fields_cannot_exceed_the_note_or_input_budget() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store, "Combined limits");
    let mut rendered = review(&p);
    rendered["checks"] = json!((0..12).map(|i| json!({"name":format!("Check {i}"),"outcome":"not_run","result":"x".repeat(1500)})).collect::<Vec<_>>());
    assert!(store.execute("save_review", rendered).is_err());
    let mut encoded = review(&p);
    encoded["sources"] = json!(vec!["界".repeat(2048); 10]);
    encoded["findings"][0]["evidence"] = json!("界".repeat(2000));
    assert!(store.execute("save_review", encoded).is_err());
    let page = store
        .execute("get_review_history", json!({"project_id":p}))
        .unwrap();
    assert_eq!(page["items"], json!([]));
}

#[tokio::test]
async fn actual_mcp_tools_roundtrip_review_and_canonical_budget() {
    use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
    use std::time::Duration;
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("review memory.sqlite");
    let mut store = Store::open(&path).unwrap();
    let p = project(&mut store, "Wire review");
    drop(store);
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"));
    command.arg("--db").arg(&path);
    let client = tokio::time::timeout(
        Duration::from_secs(10),
        ().serve(TokioChildProcess::new(command).unwrap()),
    )
    .await
    .unwrap()
    .unwrap();
    let tools = client.list_all_tools().await.unwrap();
    let save = tools
        .iter()
        .find(|tool| tool.name == "save_review")
        .unwrap();
    assert_eq!(
        save.annotations.as_ref().unwrap().read_only_hint,
        Some(false)
    );
    let read = tools
        .iter()
        .find(|tool| tool.name == "get_review_history")
        .unwrap();
    assert_eq!(
        read.annotations.as_ref().unwrap().read_only_hint,
        Some(true)
    );
    let saved = client
        .call_tool(
            CallToolRequestParams::new("save_review")
                .with_arguments(review(&p).as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_ne!(saved.is_error, Some(true));
    let page = client
        .call_tool(
            CallToolRequestParams::new("get_review_history").with_arguments(
                json!({"project_id":p,"current_head":"b".repeat(40)})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(page.is_error, Some(true));
    let value = page.structured_content.as_ref().unwrap();
    assert_eq!(
        value["items"][0]["id"],
        saved.structured_content.as_ref().unwrap()["id"]
    );
    assert_eq!(value["items"][0]["freshness"], "current");
    // Legacy protocol negotiation omits resultType on the wire. The budget is
    // defined by canonical rmcp serialization, independently rebuilt here.
    assert_eq!(
        value["budget"]["used_bytes"],
        serde_json::to_vec(&CallToolResult::structured(value.clone()))
            .unwrap()
            .len()
    );
    client.cancel().await.unwrap();
}
