use project_memory_mcp::store::Store;
use serde_json::json;
use std::sync::{Arc, Barrier};

#[test]
#[ignore = "crash fixture helper invoked by the parent WAL regression"]
fn leave_committed_wal_without_sqlite_close() {
    let path = std::env::var_os("PMCP_TEST_WAL_PATH").unwrap();
    let kind = std::env::var("PMCP_TEST_WAL_KIND").unwrap();
    let conn = rusqlite::Connection::open(std::path::Path::new(&path)).unwrap();
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;")
        .unwrap();
    conn.execute_batch(if kind == "newer" {
        "PRAGMA application_id=1347240784; PRAGMA user_version=2;"
    } else {
        "PRAGMA application_id=1234;"
    })
    .unwrap();
    conn.execute_batch("CREATE TABLE foreign_data(value); INSERT INTO foreign_data VALUES(99);")
        .unwrap();
    // Process teardown leaves committed WAL pages, unlike Connection::drop.
    std::process::exit(0);
}

#[test]
fn rejecting_unsupported_committed_wal_never_checkpoints_its_main_file() {
    let temp = tempfile::tempdir().unwrap();
    for kind in ["foreign", "newer"] {
        let path = temp.path().join(format!("{kind}.sqlite"));
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "leave_committed_wal_without_sqlite_close",
            ])
            .env("PMCP_TEST_WAL_PATH", &path)
            .env("PMCP_TEST_WAL_KIND", kind)
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        let before = std::fs::read(&path).unwrap();
        let wal = std::path::PathBuf::from(format!("{}-wal", path.display()));
        let before_wal = std::fs::read(&wal).unwrap();
        let error = Store::open(&path)
            .err()
            .expect("unsupported identity must be refused");
        assert_eq!(
            error.code(),
            if kind == "newer" {
                "NEWER_SCHEMA"
            } else {
                "FOREIGN_DATABASE"
            }
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "refusal checkpointed the main file"
        );
        assert_eq!(
            std::fs::read(wal).unwrap(),
            before_wal,
            "refusal changed committed WAL pages"
        );
    }
}

#[test]
fn initialization_does_not_restart_its_deadline_after_waiting_for_a_writer() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("empty.sqlite");
    let writer = rusqlite::Connection::open(&path).unwrap();
    writer.execute_batch("VACUUM;").unwrap();
    let reader = rusqlite::Connection::open(&path).unwrap();
    reader.execute_batch("BEGIN;").unwrap();
    reader
        .query_row("SELECT count(*) FROM sqlite_schema", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap();
    writer.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let release_writer = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(3250));
        writer.execute_batch("ROLLBACK;").unwrap();
    });
    let release_reader = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(6500));
        reader.execute_batch("ROLLBACK;").unwrap();
    });
    let result = Store::open(&path);
    release_writer.join().unwrap();
    release_reader.join().unwrap();
    assert!(
        result.is_err(),
        "a lock released after the five-second budget must not turn startup into a late success"
    );
    assert_eq!(
        Store::open(&path).unwrap().diagnostics().unwrap()["schema_version"],
        1
    );
}

#[test]
fn concurrent_first_open_serializes_identity_and_preserves_each_clients_write() {
    for round in 0..16 {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("shared.sqlite");
        let barrier = Arc::new(Barrier::new(12));
        let workers: Vec<_> = (0..12)
            .map(|worker| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    let mut store = Store::open(&path)?;
                    store.execute(
                        "create_project",
                        json!({"name":format!("Worker {worker}"),"purpose":"Concurrent synthetic startup"}),
                    )
                })
            })
            .collect();
        for worker in workers {
            assert!(
                worker.join().unwrap().is_ok(),
                "failed startup/write in round {round}"
            );
        }
        let store = Store::open_read_only(&path).unwrap();
        assert_eq!(store.diagnostics().unwrap()["project_count"], 12);
        assert_eq!(store.diagnostics().unwrap()["schema_version"], 1);
    }
}

#[test]
fn concurrent_foreign_archive_refusals_leave_its_bytes_and_identity_untouched() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("foreign.sqlite");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA application_id=1234; CREATE TABLE private_data(value TEXT); INSERT INTO private_data VALUES('synthetic');").unwrap();
    drop(conn);
    let original = std::fs::read(&path).unwrap();
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                Store::open(&path).err().unwrap().code().to_string()
            })
        })
        .collect();
    for worker in workers {
        assert_eq!(worker.join().unwrap(), "FOREIGN_DATABASE");
    }
    assert_eq!(std::fs::read(path).unwrap(), original);
}
