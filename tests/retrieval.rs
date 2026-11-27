use project_memory_mcp::store::Store;
use rmcp::model::CallToolResult;
use serde_json::{Value, json};

fn call(s: &mut Store, operation: &str, args: Value) -> Value {
    s.execute(operation, args).unwrap()
}

fn project(s: &mut Store) -> String {
    call(
        s,
        "create_project",
        json!({"name":"Toolkit", "purpose":"Retrieve evidence"}),
    )["id"]
        .as_str()
        .unwrap()
        .into()
}

fn note(s: &mut Store, project: &str, text: &str) -> Value {
    call(
        s,
        "save_note",
        json!({"project_id":project,"title":"Note","content":text,"author_type":"assistant","sources":["src/example.rs:1"]}),
    )
}

fn decision(s: &mut Store, project: &str, text: &str, supersedes: Option<&str>) -> Value {
    call(
        s,
        "record_decision",
        json!({"project_id":project,"decision":text,"rationale":"Explicitly chosen","author_type":"user","supersedes":supersedes}),
    )
}

fn handoff(s: &mut Store, project: &str, summary: &str) -> Value {
    call(
        s,
        "save_handoff",
        json!({"project_id":project,"summary":summary,"completed":["completed marker"],"unfinished":["unfinished marker"],"next_actions":["next marker"],"blockers":["blocker marker"],"verification":["verification marker"],"author_type":"assistant","sources":[]}),
    )
}

fn assert_budget(bundle: &Value, limit: usize) {
    let result = CallToolResult::structured(bundle.clone());
    let used = serde_json::to_vec(&result).unwrap().len();
    assert!(used <= limit, "{used} exceeds {limit}");
    assert_eq!(bundle["budget"]["used_bytes"], used);
    assert_eq!(bundle["budget"]["estimated_tokens"], used.div_ceil(4));
    assert!(
        bundle["budget"]["estimate_method"]
            .as_str()
            .unwrap()
            .contains("heuristic")
    );
    assert_eq!(
        result.content.len(),
        1,
        "normal text compatibility representation remains"
    );
}

#[test]
fn unified_search_matches_literal_values_in_each_kind_without_json_escape_false_matches() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    let literal = "Shared 100%_ \\path \"quoted\"\nNUL\0日本語 résumé";
    let n = note(&mut s, &id, literal);
    let d = decision(&mut s, &id, literal, None);
    let h = handoff(&mut s, &id, literal);
    for query in [
        literal,
        "%_",
        "\\path",
        "\"quoted\"",
        "\nNUL\0",
        "日本語",
        "SHARED",
        "résumé",
    ] {
        let page = call(
            &mut s,
            "search_memory",
            json!({"project_id":id,"query":query}),
        );
        assert_eq!(
            page["items"].as_array().unwrap().len(),
            3,
            "query {query:?}"
        );
        assert_eq!(page["items"][0]["id"], h["id"]);
        assert_eq!(page["items"][1]["id"], d["id"]);
        assert_eq!(page["items"][2]["id"], n["id"]);
        assert_eq!(page["items"][0]["full_record_required"], true);
    }
    for query in [
        "\\nNUL",
        "\\u0000",
        "RÉSUMÉ",
        "src/example.rs:1",
        "assistant",
    ] {
        assert_eq!(
            call(
                &mut s,
                "search_memory",
                json!({"project_id":id,"query":query})
            )["items"],
            json!([]),
            "query {query:?}"
        );
    }
    for query in [
        "completed marker",
        "unfinished marker",
        "next marker",
        "blocker marker",
        "verification marker",
    ] {
        assert_eq!(
            call(
                &mut s,
                "search_memory",
                json!({"project_id":id,"query":query})
            )["items"][0]["id"],
            h["id"]
        );
    }
    assert_eq!(
        call(
            &mut s,
            "search_memory",
            json!({"project_id":id,"query":"Explicitly chosen"})
        )["items"][0]["id"],
        d["id"]
    );
}

#[test]
fn search_filters_snapshot_supersession_and_cursors_never_cross_scopes() {
    let mut s = Store::memory().unwrap();
    let a = project(&mut s);
    let b = project(&mut s);
    let old = decision(&mut s, &a, "match old", None);
    let n = note(&mut s, &a, "match note");
    let d = decision(&mut s, &a, "match current", None);
    let first = call(
        &mut s,
        "search_memory",
        json!({"project_id":a,"query":"match","limit":1}),
    );
    decision(&mut s, &a, "replacement", old["id"].as_str());
    note(&mut s, &a, "match added after snapshot");
    let cursor = first["next_cursor"].clone();
    let rest = call(
        &mut s,
        "search_memory",
        json!({"project_id":a,"query":"match","cursor":cursor}),
    );
    assert_eq!(
        rest["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].clone())
            .collect::<Vec<_>>(),
        vec![n["id"].clone(), old["id"].clone()]
    );
    assert!(
        rest["items"][1]["superseded_by"].is_null(),
        "snapshot preserves original status"
    );
    assert_eq!(first["items"][0]["id"], d["id"]);
    let current = call(
        &mut s,
        "search_memory",
        json!({"project_id":a,"query":"match","kind":"decision"}),
    );
    assert_eq!(current["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        call(
            &mut s,
            "search_memory",
            json!({"project_id":a,"query":"match","kind":"decision","include_superseded":true})
        )["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for args in [
        json!({"project_id":b,"query":"match","cursor":cursor}),
        json!({"project_id":a,"query":"other","cursor":cursor}),
        json!({"project_id":a,"query":"match","kind":"note","cursor":cursor}),
        json!({"project_id":a,"query":"match","include_superseded":true,"cursor":cursor}),
    ] {
        assert_eq!(
            s.execute("search_memory", args).unwrap_err().code(),
            "INVALID_CURSOR"
        );
    }
    assert_eq!(
        s.execute(
            "search_notes",
            json!({"project_id":a,"query":"match","cursor":cursor})
        )
        .unwrap_err()
        .code(),
        "INVALID_CURSOR"
    );
    assert_eq!(
        call(
            &mut s,
            "search_memory",
            json!({"project_id":b,"query":"match"})
        )["items"],
        json!([])
    );
}

#[test]
fn default_bundle_prioritizes_whole_handoff_active_decisions_and_recent_notes() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    let old = decision(&mut s, &id, "old", None);
    let active = decision(&mut s, &id, "chosen", old["id"].as_str());
    handoff(&mut s, &id, "older handoff");
    let latest = handoff(&mut s, &id, "latest handoff");
    let n = note(&mut s, &id, "recent note");
    let bundle = call(&mut s, "get_context_bundle", json!({"project_id":id}));
    let records = bundle["records"].as_array().unwrap();
    assert_eq!(
        records.iter().map(|r| r["id"].clone()).collect::<Vec<_>>(),
        vec![latest["id"].clone(), active["id"].clone(), n["id"].clone()]
    );
    for (actual, full) in records.iter().zip([&latest, &active, &n]) {
        let mut actual = actual.clone();
        actual.as_object_mut().unwrap().remove("selection_reason");
        assert_eq!(&actual, full, "every provenance and content field survives");
    }
    assert_eq!(records[0]["selection_reason"], "latest_handoff");
    assert_eq!(records[1]["selection_reason"], "active_decision");
    assert_eq!(bundle["selection"]["selection_complete"], true);
    assert!(bundle["notice"].as_str().unwrap().contains("untrusted"));
    assert_budget(&bundle, 16000);
}

#[test]
fn bundle_skips_oversize_candidates_and_reports_record_cap_omissions() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    let small = note(&mut s, &id, "small complete record");
    note(&mut s, &id, &"\"\\\n😀".repeat(3000));
    let bundle = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"max_bytes":4096}),
    );
    assert_eq!(bundle["records"].as_array().unwrap().len(), 1);
    assert_eq!(bundle["records"][0]["id"], small["id"]);
    assert_eq!(bundle["selection"]["omitted_by_budget"], 1);
    assert_eq!(bundle["selection"]["selection_complete"], false);
    assert_budget(&bundle, 4096);
    note(&mut s, &id, "new small");
    let capped = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"max_records":1,"max_bytes":65536}),
    );
    assert_eq!(capped["records"].as_array().unwrap().len(), 1);
    assert_eq!(capped["selection"]["omitted_by_record_limit"], 2);
    assert_budget(&capped, 65536);
}

#[test]
fn final_omission_accounting_reconsiders_record_limit_candidates_after_eviction() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    let mut small = Value::Null;
    for _ in 0..10 {
        small = note(&mut s, &id, "small");
    }
    let newest = note(&mut s, &id, &"x".repeat(1000));
    let generous = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"max_bytes":5000,"max_records":1}),
    );
    assert_eq!(generous["records"][0]["id"], newest["id"]);
    let limit = generous["budget"]["used_bytes"].as_u64().unwrap() as usize - 1;
    let explicit = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"record_ids":[small["id"]],"max_bytes":limit,"max_records":1}),
    );
    assert_budget(&explicit, limit);
    let bounded = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"max_bytes":limit,"max_records":1}),
    );
    assert_eq!(
        bounded["records"].as_array().unwrap().len(),
        1,
        "after final metadata displaces the large record, a smaller candidate must still be considered"
    );
    assert_eq!(bounded["records"][0]["id"], small["id"]);
    assert_eq!(bounded["records"][0]["content"], "small");
    assert_eq!(bounded["selection"]["candidate_count"], 11);
    assert_eq!(bounded["selection"]["omitted_by_budget"], 1);
    assert_eq!(bounded["selection"]["omitted_by_record_limit"], 9);
    assert_eq!(bounded["selection"]["selection_complete"], false);
    assert_eq!(
        bounded["records"].as_array().unwrap().len() as u64
            + bounded["selection"]["omitted_by_budget"].as_u64().unwrap()
            + bounded["selection"]["omitted_by_record_limit"]
                .as_u64()
                .unwrap(),
        bounded["selection"]["candidate_count"].as_u64().unwrap()
    );
    assert_budget(&bounded, limit);
}

#[test]
fn metadata_repair_skips_another_large_candidate_before_refilling_from_query_matches() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    let mut small = Value::Null;
    for _ in 0..10 {
        small = note(&mut s, &id, "small");
    }
    note(&mut s, &id, &"x".repeat(2000));
    let newest = note(&mut s, &id, &"x".repeat(1000));
    let generous = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"query":"Note","max_bytes":5000,"max_records":1}),
    );
    assert_eq!(generous["records"][0]["id"], newest["id"]);
    let limit = generous["budget"]["used_bytes"].as_u64().unwrap() as usize - 1;
    let bounded = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"query":"Note","max_bytes":limit,"max_records":1}),
    );
    assert_eq!(bounded["records"][0]["id"], small["id"]);
    assert_eq!(bounded["selection"]["candidate_count"], 12);
    assert_eq!(bounded["selection"]["omitted_by_budget"], 2);
    assert_eq!(bounded["selection"]["omitted_by_record_limit"], 9);
    assert_eq!(
        bounded["records"][0]["selection_reason"],
        "literal_query_match"
    );
    assert_budget(&bounded, limit);
}

#[test]
fn explicit_bundle_is_atomic_project_scoped_and_preserves_requested_history_order() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    let other = project(&mut s);
    let old = decision(&mut s, &id, "old", None);
    let new = decision(&mut s, &id, "new", old["id"].as_str());
    let foreign = note(&mut s, &other, "foreign");
    let huge = note(&mut s, &id, &"x".repeat(16000));
    let bundle = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"record_ids":[old["id"],new["id"]]}),
    );
    assert_eq!(bundle["records"][0]["id"], old["id"]);
    assert_eq!(bundle["records"][0]["superseded_by"], new["id"]);
    assert_eq!(
        bundle["records"][0]["selection_reason"],
        "explicit_selection"
    );
    assert_budget(&bundle, 16000);
    for (args, code) in [
        (
            json!({"project_id":id,"record_ids":[old["id"],foreign["id"]]}),
            "NOT_FOUND",
        ),
        (
            json!({"project_id":id,"record_ids":[old["id"],"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"]}),
            "NOT_FOUND",
        ),
        (
            json!({"project_id":id,"record_ids":[old["id"],old["id"]]}),
            "VALIDATION_ERROR",
        ),
        (json!({"project_id":id,"record_ids":[]}), "VALIDATION_ERROR"),
        (
            json!({"project_id":id,"record_ids":[old["id"],new["id"]],"max_records":1}),
            "VALIDATION_ERROR",
        ),
        (
            json!({"project_id":id,"record_ids":[old["id"]],"query":"old"}),
            "VALIDATION_ERROR",
        ),
        (
            json!({"project_id":id,"record_ids":[old["id"],huge["id"]],"max_bytes":4096}),
            "BUDGET_EXCEEDED",
        ),
    ] {
        assert_eq!(
            s.execute("get_context_bundle", args).unwrap_err().code(),
            code
        );
    }
}

#[test]
fn query_bundle_only_includes_literal_matching_current_records() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    let old = decision(&mut s, &id, "needle old", None);
    decision(&mut s, &id, "different", old["id"].as_str());
    let d = decision(&mut s, &id, "needle current", None);
    let h = handoff(&mut s, &id, "needle handoff");
    note(&mut s, &id, "unrelated");
    let bundle = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"query":"NEEDLE"}),
    );
    assert_eq!(bundle["selection"]["mode"], "query");
    assert_eq!(
        bundle["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].clone())
            .collect::<Vec<_>>(),
        vec![h["id"].clone(), d["id"].clone()]
    );
    assert_eq!(
        bundle["records"][0]["selection_reason"],
        "literal_query_match"
    );
    assert_budget(&bundle, 16000);
}

#[test]
fn automatic_bundle_window_is_bounded_and_exposes_incomplete_candidate_pool() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    for _ in 0..203 {
        note(&mut s, &id, "small");
    }
    let bundle = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"max_records":1}),
    );
    assert_eq!(bundle["selection"]["candidate_count"], 200);
    assert_eq!(bundle["selection"]["candidate_window_truncated"], true);
    assert_eq!(bundle["selection"]["omitted_by_record_limit"], 199);
    assert_eq!(bundle["selection"]["selection_complete"], false);
    assert_budget(&bundle, 16000);
}

#[test]
fn bundles_reject_invalid_limits_unknown_fields_and_unfit_project_headers() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    for extra in [
        json!({"max_bytes":4095}),
        json!({"max_bytes":65537}),
        json!({"max_records":0}),
        json!({"max_records":21}),
        json!({"query":" "}),
        json!({"typo":1}),
    ] {
        let mut args = json!({"project_id":id});
        args.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert_eq!(
            s.execute("get_context_bundle", args).unwrap_err().code(),
            "VALIDATION_ERROR"
        );
    }
    let huge = call(
        &mut s,
        "create_project",
        json!({"name":"Large header","purpose":"😀".repeat(4000)}),
    );
    assert_eq!(
        s.execute(
            "get_context_bundle",
            json!({"project_id":huge["id"],"max_bytes":4096})
        )
        .unwrap_err()
        .code(),
        "BUDGET_EXCEEDED"
    );
    let empty = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"query":"no matches"}),
    );
    assert_eq!(empty["records"], json!([]));
    assert_eq!(empty["selection"]["selection_complete"], true);
    assert_budget(&empty, 16000);
}

#[test]
fn read_only_store_rejects_all_mutations_before_validating_input_or_sql() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("memory.sqlite");
    let mut writable = Store::open(&path).unwrap();
    let id = project(&mut writable);
    note(&mut writable, &id, "readable");
    drop(writable);
    let mut s = Store::open_read_only(&path).unwrap();
    for operation in [
        "create_project",
        "save_note",
        "record_decision",
        "save_handoff",
        "delete_project",
    ] {
        assert_eq!(
            s.execute(operation, json!({})).unwrap_err().code(),
            "READ_ONLY",
            "{operation}"
        );
    }
    assert_eq!(
        call(
            &mut s,
            "search_memory",
            json!({"project_id":id,"query":"readable"})
        )["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_budget(
        &call(&mut s, "get_context_bundle", json!({"project_id":id})),
        16000,
    );
}

#[test]
fn terms_bundle_matches_distributed_fields_and_preserves_literal_default() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    let strongest = call(
        &mut s,
        "save_note",
        json!({"project_id":id,"title":"Windows path","content":"Authentication behavior and complete original evidence.","author_type":"user","sources":["src/path.rs:42"]}),
    );
    let weak = note(&mut s, &id, "authentication only");
    let literal = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"query":"windows authentication"}),
    );
    assert_eq!(literal["records"], json!([]));
    assert_eq!(literal["selection"]["mode"], "query");
    assert!(literal["selection"].get("query_terms").is_none());
    assert!(literal["selection"].get("ranking").is_none());
    let explicit_literal = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"query":"windows authentication","query_mode":"literal"}),
    );
    assert_eq!(explicit_literal, literal);
    let bundle = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"query":"windows authentication","query_mode":"terms"}),
    );
    assert_eq!(bundle["selection"]["mode"], "terms");
    assert_eq!(
        bundle["selection"]["query_terms"],
        json!(["windows", "authentication"])
    );
    assert_eq!(
        bundle["selection"]["ranking"],
        "term_coverage_then_headline_then_recency"
    );
    assert_eq!(bundle["records"][0]["id"], strongest["id"]);
    assert_eq!(
        bundle["records"][0]["matched_terms"],
        json!(["windows", "authentication"])
    );
    assert_eq!(
        bundle["records"][0]["selection_reason"],
        "ranked_term_match"
    );
    assert_eq!(bundle["records"][0]["content"], strongest["content"]);
    assert_eq!(bundle["records"][0]["sources"], strongest["sources"]);
    assert_eq!(bundle["records"][1]["id"], weak["id"]);
    assert_budget(&bundle, 16000);
}

#[test]
fn terms_ranking_precedes_the_candidate_window() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    let strong = note(&mut s, &id, "windows authentication");
    for _ in 0..205 {
        note(&mut s, &id, "windows unrelated recent activity");
    }
    let bundle = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"query":"authentication windows","query_mode":"terms","max_records":1}),
    );
    assert_eq!(bundle["records"][0]["id"], strong["id"]);
    assert_eq!(bundle["selection"]["candidate_count"], 200);
    assert_eq!(bundle["selection"]["candidate_window_truncated"], true);
    assert_eq!(bundle["selection"]["omitted_by_record_limit"], 199);
    assert_eq!(bundle["selection"]["selection_complete"], false);
    assert_budget(&bundle, 16000);
}

#[test]
fn terms_ranking_prefers_headline_coverage_then_insertion_recency() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    let old_headline = call(
        &mut s,
        "save_note",
        json!({"project_id":id,"title":"alpha beta","content":"Original full evidence","author_type":"user"}),
    );
    let new_headline = decision(&mut s, &id, "alpha beta", None);
    let body_only = note(&mut s, &id, "alpha beta");
    let bundle = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"query":"alpha beta","query_mode":"terms"}),
    );
    let ids = bundle["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["id"].clone())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            new_headline["id"].clone(),
            old_headline["id"].clone(),
            body_only["id"].clone()
        ]
    );
}

#[test]
fn terms_use_ordered_unicode_alphanumeric_runs_and_ascii_only_folding() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    let evidence = note(&mut s, &id, "rust 日本語 CAFÉ café");
    let bundle = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"query":"RUST-rust_日本語; CAFÉ café","query_mode":"terms"}),
    );
    assert_eq!(
        bundle["selection"]["query_terms"],
        json!(["rust", "日本語", "cafÉ", "café"])
    );
    assert_eq!(bundle["records"][0]["id"], evidence["id"]);
    assert_eq!(
        bundle["records"][0]["matched_terms"],
        bundle["selection"]["query_terms"]
    );
    let only_upper = note(&mut s, &id, "ÉLAN");
    assert_eq!(
        call(
            &mut s,
            "get_context_bundle",
            json!({"project_id":id,"query":"élan","query_mode":"terms"})
        )["records"],
        json!([])
    );
    assert_eq!(
        call(
            &mut s,
            "get_context_bundle",
            json!({"project_id":id,"query":"Élan","query_mode":"terms"})
        )["records"][0]["id"],
        only_upper["id"]
    );
    assert_budget(&bundle, 16000);
}

#[test]
fn terms_require_one_to_eight_distinct_terms_and_a_query_without_explicit_ids() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    let record = note(&mut s, &id, "alpha");
    for args in [
        json!({"project_id":id,"query_mode":"terms"}),
        json!({"project_id":id,"query_mode":"terms","record_ids":[record["id"]]}),
        json!({"project_id":id,"query":"alpha","query_mode":"terms","record_ids":[record["id"]]}),
        json!({"project_id":id,"query":"%_ - () '\" \\ \n","query_mode":"terms"}),
        json!({"project_id":id,"query":"one two three four five six seven eight nine","query_mode":"terms"}),
        json!({"project_id":id,"query":"alpha","query_mode":"semantic"}),
        json!({"project_id":id,"query":"x".repeat(2001),"query_mode":"terms"}),
    ] {
        assert_eq!(
            s.execute("get_context_bundle", args).unwrap_err().code(),
            "VALIDATION_ERROR"
        );
    }
    let bundle = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"query":"alpha ALPHA alpha alpha ALPHA alpha alpha ALPHA alpha","query_mode":"terms"}),
    );
    assert_eq!(bundle["selection"]["query_terms"], json!(["alpha"]));
    assert_eq!(bundle["records"][0]["id"], record["id"]);
}

#[test]
fn terms_only_match_human_text_in_current_records_of_the_selected_project() {
    let mut s = Store::memory().unwrap();
    let a = project(&mut s);
    let b = project(&mut s);
    let old = decision(&mut s, &a, "historicalneedle", None);
    decision(&mut s, &a, "Replacement", old["id"].as_str());
    note(&mut s, &b, "foreignneedle");
    let source_only = call(
        &mut s,
        "save_note",
        json!({"project_id":a,"title":"ordinary","content":"ordinary","author_type":"assistant","sources":["sourceneedle"]}),
    );
    for query in [
        "historicalneedle",
        "foreignneedle",
        "sourceneedle",
        "assistant",
        source_only["id"].as_str().unwrap(),
    ] {
        assert_eq!(
            call(
                &mut s,
                "get_context_bundle",
                json!({"project_id":a,"query":query,"query_mode":"terms"})
            )["records"],
            json!([]),
            "query {query}"
        );
    }
    let rationale = call(
        &mut s,
        "record_decision",
        json!({"project_id":a,"decision":"Current choice","rationale":"rationaleneedle","author_type":"user"}),
    );
    let lists = call(
        &mut s,
        "save_handoff",
        json!({"project_id":a,"summary":"Present work","completed":[],"unfinished":["unfinishedneedle"],"next_actions":[],"blockers":[],"verification":["verificationneedle"],"author_type":"assistant"}),
    );
    let bundle = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":a,"query":"rationaleneedle unfinishedneedle verificationneedle","query_mode":"terms"}),
    );
    assert_eq!(bundle["records"][0]["id"], lists["id"]);
    assert_eq!(
        bundle["records"][0]["matched_terms"],
        json!(["unfinishedneedle", "verificationneedle"])
    );
    assert_eq!(bundle["records"][1]["id"], rationale["id"]);
    let all_lists = handoff(&mut s, &a, "Distinct summary");
    let lists_bundle = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":a,"query":"completed unfinished next blocker verification","query_mode":"terms"}),
    );
    assert_eq!(lists_bundle["records"][0]["id"], all_lists["id"]);
    assert_eq!(
        lists_bundle["records"][0]["matched_terms"],
        json!(["completed", "unfinished", "next", "blocker", "verification"])
    );
}

#[test]
fn terms_budget_counts_ranking_metadata_and_skips_complete_oversized_candidates() {
    let mut s = Store::memory().unwrap();
    let id = project(&mut s);
    let small = note(&mut s, &id, "alpha complete small café 日本語 \\\"");
    note(&mut s, &id, &format!("alpha beta {}", "x".repeat(5000)));
    let bundle = call(
        &mut s,
        "get_context_bundle",
        json!({"project_id":id,"query":"alpha beta","query_mode":"terms","max_bytes":4096}),
    );
    assert_eq!(bundle["records"].as_array().unwrap().len(), 1);
    assert_eq!(bundle["records"][0]["id"], small["id"]);
    assert_eq!(bundle["records"][0]["content"], small["content"]);
    assert_eq!(bundle["selection"]["omitted_by_budget"], 1);
    assert_budget(&bundle, 4096);
}

#[test]
fn terms_bundle_operates_on_read_only_archives() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("memory.sqlite");
    let mut writer = Store::open(&path).unwrap();
    let id = project(&mut writer);
    let record = note(&mut writer, &id, "alpha beta");
    drop(writer);
    let mut reader = Store::open_read_only(&path).unwrap();
    let bundle = call(
        &mut reader,
        "get_context_bundle",
        json!({"project_id":id,"query":"beta alpha","query_mode":"terms"}),
    );
    assert_eq!(bundle["records"][0]["id"], record["id"]);
    assert_budget(&bundle, 16000);
}
