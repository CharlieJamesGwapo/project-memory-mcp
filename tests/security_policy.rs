use project_memory_mcp::{security::AccessPolicy, store::Store};
use serde_json::{Value, json};

fn project(store: &mut Store, name: &str) -> String {
    store
        .execute(
            "create_project",
            json!({"name":name,"purpose":"Synthetic security fixture"}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}

#[test]
fn deletion_is_disabled_unless_local_policy_enables_it() {
    let mut store = Store::memory().unwrap();
    let id = project(&mut store, "Keep");
    assert_eq!(
        store
            .execute("delete_project", json!({"project_id":id,"confirm":true}))
            .unwrap_err()
            .code(),
        "ACCESS_DENIED"
    );
    store
        .set_policy(serde_json::from_value(json!({"allow_delete":true})).unwrap())
        .unwrap();
    assert_eq!(
        store
            .execute("delete_project", json!({"project_id":id,"confirm":true}))
            .unwrap()["deleted"],
        true
    );
}

#[test]
fn project_and_operation_policy_applies_to_direct_calls_and_pagination() {
    let mut store = Store::memory().unwrap();
    let a = project(&mut store, "Visible A");
    let hidden = project(&mut store, "Hidden");
    let b = project(&mut store, "Visible B");
    store.set_policy(serde_json::from_value(json!({"projects":[a,b],"operations":["list_projects","get_project_context","get_record","get_access_policy"]})).unwrap()).unwrap();
    let page = store.execute("list_projects", json!({"limit":1})).unwrap();
    assert_eq!(page["items"][0]["id"], b);
    let next = store
        .execute(
            "list_projects",
            json!({"limit":1,"cursor":page["next_cursor"]}),
        )
        .unwrap();
    assert_eq!(next["items"][0]["id"], a);
    assert!(next["next_cursor"].is_null());
    assert!(!serde_json::to_string(&page).unwrap().contains("Hidden"));
    for (op, args) in [
        ("get_project_context", json!({"project_id":hidden})),
        (
            "save_note",
            json!({"project_id":a,"title":"x","content":"x","author_type":"user"}),
        ),
        ("create_project", json!({"name":"x","purpose":"x"})),
        ("studio_list_projects", json!({})),
        ("inspect_secrets", json!({"project_id":a})),
    ] {
        assert_eq!(
            store.execute(op, args).unwrap_err().code(),
            "ACCESS_DENIED",
            "{op}"
        );
    }
    let d = store.diagnostics().unwrap();
    assert_eq!(d["project_count"], 2);
}

#[test]
fn credentials_are_rejected_without_echoing_values_and_can_be_inspected_on_opt_in() {
    let mut store = Store::memory().unwrap();
    let p = project(&mut store, "Secrets");
    let secret = format!("ghp_{}", "A".repeat(36));
    let args = json!({"project_id":p,"title":"Credential fixture","content":format!("Token {secret}"),"author_type":"user"});
    let error = store.execute("save_note", args.clone()).unwrap_err();
    assert_eq!(error.code(), "SECRET_DETECTED");
    assert!(!error.to_string().contains(&secret));
    store
        .set_policy(serde_json::from_value(json!({"secrets":"allow"})).unwrap())
        .unwrap();
    let saved = store.execute("save_note", args).unwrap();
    store.set_policy(AccessPolicy::default()).unwrap();
    let report = store
        .execute("inspect_secrets", json!({"project_id":p}))
        .unwrap();
    assert_eq!(report["items"][0]["record_id"], saved["id"]);
    assert!(!serde_json::to_string(&report).unwrap().contains(&secret));
    assert_eq!(
        store
            .execute("export_project", json!({"project_id":p}))
            .unwrap_err()
            .code(),
        "SECRET_DETECTED"
    );
    assert_eq!(
        store
            .execute(
                "get_record",
                json!({"project_id":p,"record_id":saved["id"]})
            )
            .unwrap()["content"],
        format!("Token {secret}")
    );
}

#[test]
fn policy_is_strict_and_scanning_covers_nested_values_without_treating_prose_as_secrets() {
    for value in [
        json!({"surprise":true}),
        json!({"operations":["execute_sql"]}),
        json!({"projects":["bad"]}),
        json!({"secrets":"off"}),
    ] {
        let parsed = serde_json::from_value::<AccessPolicy>(value);
        assert!(parsed.is_err() || parsed.unwrap().validate().is_err());
    }
    let ordinary: Value =
        json!({"text":"Use an API key from your environment; ghp_example is a placeholder."});
    assert!(project_memory_mcp::security::inspect(&ordinary).is_empty());
    let pem = json!({"review":{"evidence":(["-----BEGIN ","PRIVATE KEY-----\nsynthetic\n-----END ","PRIVATE KEY-----"].concat())}});
    let matches = project_memory_mcp::security::inspect(&pem);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].category, "private_key");
    assert_eq!(matches[0].path, "$.review.evidence");
}

#[test]
fn audit_contains_metadata_only_and_refuses_existing_unrelated_files() {
    let temp = tempfile::tempdir().unwrap();
    let audit = temp.path().join("audit.jsonl");
    let mut store = Store::memory().unwrap();
    store
        .set_policy(serde_json::from_value(json!({"audit_path":audit})).unwrap())
        .unwrap();
    let id = project(&mut store, "Unlogged project title");
    let secret = format!("ghp_{}", "A".repeat(36));
    assert!(store.execute("save_note",json!({"project_id":id,"title":"Private title","content":secret,"author_type":"user"})).is_err());
    let log = std::fs::read_to_string(&audit).unwrap();
    assert!(log.contains("secret_rejected"));
    assert!(!log.contains(&secret));
    assert!(!log.contains("Private title"));
    assert!(!log.contains("Unlogged project title"));
    for line in log.lines() {
        let _: Value = serde_json::from_str(line).unwrap();
    }
    let foreign = temp.path().join("unrelated.txt");
    std::fs::write(&foreign, b"Keep exact bytes").unwrap();
    assert!(
        store
            .set_policy(serde_json::from_value(json!({"audit_path":foreign})).unwrap())
            .is_err()
    );
    assert_eq!(std::fs::read(&foreign).unwrap(), b"Keep exact bytes");
}

#[test]
fn invalid_policy_fails_before_archive_creation_and_generated_config_preserves_policy() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("missing.sqlite");
    let policy = temp.path().join("policy.json");
    std::fs::write(&policy, b"{\"operations\":[\"execute_sql\"]}").unwrap();
    let run = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"))
            .args([
                "--db",
                db.to_str().unwrap(),
                "--policy",
                policy.to_str().unwrap(),
            ])
            .args(args)
            .output()
            .unwrap()
    };
    let out = run(&["init"]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(!db.exists());
    std::fs::write(&policy, b"{\"operations\":[]}").unwrap();
    let out = run(&["--read-only", "print-config"]);
    assert!(out.status.success());
    let config: Value = serde_json::from_slice(&out.stdout).unwrap();
    let args = config["mcpServers"]["project-memory"]["args"]
        .as_array()
        .unwrap();
    assert!(
        args.windows(2)
            .any(|pair| pair == [json!("--policy"), json!(policy)])
    );
    assert!(args.contains(&json!("--read-only")));
    assert!(!db.exists());
    let out = run(&["policy"]);
    assert!(out.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["operations"],
        json!([])
    );
}

#[test]
fn empty_project_scope_exposes_no_projects_records_or_creation() {
    let mut store = Store::memory().unwrap();
    let id = project(&mut store, "Hidden");
    store
        .set_policy(serde_json::from_value(json!({"projects":[]})).unwrap())
        .unwrap();
    assert_eq!(
        store.execute("list_projects", json!({})).unwrap()["items"],
        json!([])
    );
    assert_eq!(store.diagnostics().unwrap()["project_count"], 0);
    assert!(!store.allows_operation("create_project"));
    assert_eq!(
        store
            .execute("get_record", json!({"project_id":id,"record_id":id}))
            .unwrap_err()
            .code(),
        "ACCESS_DENIED"
    );
}

#[cfg(unix)]
#[test]
fn symlink_policy_and_audit_are_refused_without_changing_targets() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target");
    let link = temp.path().join("link");
    std::fs::write(&target, b"{}").unwrap();
    symlink(&target, &link).unwrap();
    assert!(AccessPolicy::load(&link).is_err());
    assert!(project_memory_mcp::security::AuditLog::open(&link).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"{}");
}

#[test]
fn rejected_audit_destination_never_creates_the_archive() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("missing.sqlite");
    let audit = temp.path().join("unrelated.txt");
    std::fs::write(&audit, b"Unrelated bytes").unwrap();
    let policy: AccessPolicy = serde_json::from_value(json!({"audit_path":audit})).unwrap();
    assert!(Store::open_with_policy(&db, false, policy).is_err());
    assert!(!db.exists());
    assert_eq!(std::fs::read(audit).unwrap(), b"Unrelated bytes");
    let policy: AccessPolicy = serde_json::from_value(json!({"audit_path":db})).unwrap();
    assert!(Store::open_with_policy(&db, false, policy).is_err());
    assert!(!db.exists());
}

#[test]
fn audit_refuses_sqlite_sidecars_before_opening_or_reconfiguration() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("memory.sqlite");
    for suffix in ["-wal", "-shm", "-journal"] {
        let audit = temp.path().join(format!("memory.sqlite{suffix}"));
        let policy: AccessPolicy = serde_json::from_value(json!({"audit_path":audit})).unwrap();
        assert!(Store::open_with_policy(&db, false, policy.clone()).is_err());
        assert!(!db.exists());
        assert!(!audit.exists());
        let mut store = Store::open(&db).unwrap();
        assert!(store.set_policy(policy).is_err());
        drop(store);
        std::fs::remove_file(&db).unwrap();
    }
}

#[test]
fn audit_refuses_case_and_unicode_aliases_of_missing_archive_sidecars() {
    let temp = tempfile::tempdir().unwrap();
    for (archive, alias) in [
        ("memory.sqlite", "MEMORY.SQLITE"),
        ("memory.sqlite", "memory.sqlite-WAL"),
        ("memory.sqlite", "memory.sqlite-SHM"),
        ("memory.sqlite", "memory.sqlite-JOURNAL"),
        ("mémory.sqlite", "me\u{301}mory.sqlite-WAL"),
    ] {
        let db = temp.path().join(archive);
        let audit = temp.path().join(alias);
        let policy: AccessPolicy = serde_json::from_value(json!({"audit_path":audit})).unwrap();
        assert!(
            Store::open_with_policy(&db, false, policy).is_err(),
            "{alias}"
        );
        assert!(!db.exists());
        assert!(!audit.exists());
    }
}
