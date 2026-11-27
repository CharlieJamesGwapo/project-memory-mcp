use project_memory_mcp::{security::AccessPolicy, setup, store::Store};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

// Snapshot paths and byte digests, so an accidental initializer or audit open is
// visible without dumping archive contents into a failed assertion.
fn tree(path: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.is_dir() {
                result.insert(path.strip_prefix(root).unwrap().into(), None);
                visit(root, &path, result);
            } else if metadata.is_file() {
                result.insert(
                    path.strip_prefix(root).unwrap().into(),
                    Some(Sha256::digest(fs::read(&path).unwrap()).to_vec()),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(path, path, &mut result);
    result
}

fn executable(root: &Path) -> PathBuf {
    let path = root.join("server executable with spaces");
    // Deliberately not a runnable program: preview must inspect metadata only.
    fs::write(&path, b"this is not executable code").unwrap();
    path
}

fn check<'a>(report: &'a Value, code: &str) -> &'a Value {
    report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["code"] == code)
        .unwrap()
}

fn unchanged_except_sqlite_coordination(
    mut before: BTreeMap<PathBuf, Option<Vec<u8>>>,
    mut after: BTreeMap<PathBuf, Option<Vec<u8>>>,
    database: &Path,
) {
    let name = database.file_name().unwrap().to_string_lossy();
    let shm = PathBuf::from(format!("{name}-shm"));
    before.remove(&shm);
    after.remove(&shm);
    let wal = PathBuf::from(format!("{name}-wal"));
    if !before.contains_key(&wal)
        && let Some(created) = after.remove(&wal)
    {
        assert_eq!(
            created,
            Some(Sha256::digest([]).to_vec()),
            "a new coordination WAL must be empty"
        );
    }
    assert_eq!(
        after, before,
        "main archive, existing WAL, executable, policy, and other paths must remain unchanged"
    );
}

#[test]
fn missing_archive_is_actionable_without_creating_any_path() {
    let temp = tempfile::tempdir().unwrap();
    let binary = executable(temp.path());
    let database = temp.path().join("not created/memory.sqlite");
    let before = tree(temp.path());
    let report = setup::preview(&database, &binary, &AccessPolicy::default(), None, false).unwrap();
    assert_eq!(report["status"], "action_required");
    assert_eq!(
        check(&report, "archive_missing")["status"],
        "action_required"
    );
    assert_eq!(
        report["configuration"]["mcpServers"]["project-memory"]["args"],
        json!(["--db", database])
    );
    assert!(!report["next_actions"].as_array().unwrap().is_empty());
    assert_eq!(tree(temp.path()), before);
}

#[test]
fn existing_archive_preserves_policy_scope_audit_and_exact_json_arguments() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("memory with spaces.sqlite");
    let binary = executable(temp.path());
    let mut store = Store::open(&database).unwrap();
    let project = store
        .execute(
            "create_project",
            json!({"name":"private text must not appear","purpose":"synthetic"}),
        )
        .unwrap();
    store
        .execute(
            "create_project",
            json!({"name":"outside scope","purpose":"synthetic"}),
        )
        .unwrap();
    let project_id = project["project"]["id"]
        .as_str()
        .unwrap_or_else(|| project["id"].as_str().unwrap());
    let policy = AccessPolicy {
        projects: Some(vec![project_id.into()]),
        operations: Some(vec!["get_record".into()]),
        audit_path: Some(temp.path().join("missing audit/metadata.jsonl")),
        ..Default::default()
    };
    let policy_path = temp.path().join("policy with spaces.json");
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    drop(store);
    let before = tree(temp.path());
    let report = setup::preview(&database, &binary, &policy, Some(&policy_path), true).unwrap();
    assert_eq!(report["status"], "ready");
    assert_eq!(report["policy"]["audit_enabled"], true);
    assert_eq!(report["policy"]["operations"], json!(["get_record"]));
    assert_eq!(report["diagnostics"]["project_count"], 1);
    assert_eq!(
        report["configuration"],
        json!({"mcpServers":{"project-memory":{"command":binary,"args":["--db",database,"--policy",policy_path,"--read-only"]}}})
    );
    assert_eq!(check(&report, "connection_not_tested")["status"], "passed");
    assert!(report["notice"].as_str().unwrap().contains("not tested"));
    assert!(!report.to_string().contains("private text must not appear"));
    unchanged_except_sqlite_coordination(before, tree(temp.path()), &database);
    assert!(!policy.audit_path.unwrap().exists());
}

#[test]
fn foreign_and_newer_archives_keep_main_and_wal_bytes_with_only_shm_coordination() {
    let temp = tempfile::tempdir().unwrap();
    let binary = executable(temp.path());
    for (application, version, expected) in [
        (7, 1, "archive_foreign"),
        (0x504d4350, 2, "archive_newer_schema"),
    ] {
        let database = temp.path().join(format!("{application}-{version}.sqlite"));
        let connection = rusqlite::Connection::open(&database).unwrap();
        connection.execute_batch(&format!("PRAGMA journal_mode=WAL; PRAGMA application_id={application}; PRAGMA user_version={version}; CREATE TABLE private(value TEXT); INSERT INTO private VALUES ('do not disclose this text');")).unwrap();
        let before = tree(temp.path());
        let report =
            setup::preview(&database, &binary, &AccessPolicy::default(), None, false).unwrap();
        assert_eq!(report["status"], "action_required");
        assert_eq!(check(&report, expected)["status"], "action_required");
        assert!(!report.to_string().contains("do not disclose this text"));
        unchanged_except_sqlite_coordination(before, tree(temp.path()), &database);
        drop(connection);
    }
}

#[test]
fn missing_executable_is_actionable_and_config_is_not_copyable() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("memory.sqlite");
    let binary = temp.path().join("not installed/server");
    let before = tree(temp.path());
    let report = setup::preview(&database, &binary, &AccessPolicy::default(), None, false).unwrap();
    assert_eq!(
        check(&report, "executable_missing")["status"],
        "action_required"
    );
    assert!(report["configuration"].is_null());
    assert_eq!(tree(temp.path()), before);
}

#[test]
fn relative_and_directory_startup_paths_are_rejected_without_changes() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("memory.sqlite");
    let binary = executable(temp.path());
    let before = tree(temp.path());
    for (db, exe, policy) in [
        (Path::new("relative.sqlite"), binary.as_path(), None),
        (database.as_path(), Path::new("relative-server"), None),
        (database.as_path(), temp.path(), None),
        (
            database.as_path(),
            binary.as_path(),
            Some(Path::new("relative-policy.json")),
        ),
        (temp.path(), binary.as_path(), None),
    ] {
        let report = setup::preview(db, exe, &AccessPolicy::default(), policy, false).unwrap();
        assert_eq!(report["status"], "action_required");
        assert!(report["configuration"].is_null());
    }
    assert_eq!(tree(temp.path()), before);
}

#[cfg(unix)]
#[test]
fn symlink_startup_files_are_rejected_but_parent_aliases_preserve_the_explicit_path() {
    let temp = tempfile::tempdir().unwrap();
    let real = fs::canonicalize(temp.path()).unwrap();
    let database = real.join("memory.sqlite");
    drop(Store::open(&database).unwrap());
    let binary = executable(&real);
    let link = real.join("alias");
    std::os::unix::fs::symlink(&binary, &link).unwrap();
    let report = setup::preview(&database, &link, &AccessPolicy::default(), None, false).unwrap();
    assert_eq!(report["status"], "action_required");
    assert_eq!(
        check(&report, "executable_invalid")["status"],
        "action_required"
    );
    assert!(report["configuration"].is_null());
    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&database, &link).unwrap();
    let report = setup::preview(&link, &binary, &AccessPolicy::default(), None, false).unwrap();
    assert_eq!(report["status"], "action_required");
    assert_eq!(
        check(&report, "database_path_invalid")["status"],
        "action_required"
    );
    assert!(report["configuration"].is_null());
    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let report = setup::preview(
        &link.join("missing.sqlite"),
        &binary,
        &AccessPolicy::default(),
        None,
        false,
    )
    .unwrap();
    assert_eq!(
        report["configuration"]["mcpServers"]["project-memory"]["args"],
        json!(["--db", link.join("missing.sqlite")])
    );
}

#[test]
fn invalid_policy_is_reported_and_never_downgraded_to_unrestricted_config() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("memory.sqlite");
    let binary = executable(temp.path());
    let policy = AccessPolicy {
        operations: Some(vec!["unknown_operation".into()]),
        ..Default::default()
    };
    let report = setup::preview(&database, &binary, &policy, None, false).unwrap();
    assert_eq!(
        check(&report, "policy_invalid")["status"],
        "action_required"
    );
    assert!(report["configuration"].is_null());
    assert!(!report.to_string().contains("unknown_operation"));
    let blocked = AccessPolicy {
        operations: Some(vec![]),
        ..Default::default()
    };
    let report = setup::preview(&database, &binary, &blocked, None, false).unwrap();
    assert!(
        report["configuration"].is_null(),
        "a restriction without a policy file cannot become unrestricted copied configuration"
    );
}

#[test]
fn policy_file_mismatch_and_archive_audit_alias_are_actionable_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("memory.sqlite");
    let binary = executable(temp.path());
    let policy_path = temp.path().join("policy.json");
    fs::write(&policy_path, b"{}").unwrap();
    let policy = AccessPolicy {
        operations: Some(vec![]),
        ..Default::default()
    };
    let before = tree(temp.path());
    let report = setup::preview(&database, &binary, &policy, Some(&policy_path), false).unwrap();
    assert!(report["configuration"].is_null());
    let alias = AccessPolicy {
        audit_path: Some(database.clone()),
        ..Default::default()
    };
    fs::write(&policy_path, serde_json::to_vec(&alias).unwrap()).unwrap();
    let alias_before = tree(temp.path());
    let report = setup::preview(&database, &binary, &alias, Some(&policy_path), false).unwrap();
    assert_eq!(report["status"], "action_required");
    assert!(report["configuration"].is_null());
    assert_eq!(tree(temp.path()), alias_before);
    assert_eq!(before.len(), alias_before.len());
}
