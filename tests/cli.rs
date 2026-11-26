use std::process::Command;

#[test]
fn help_and_version_do_not_start_a_server_or_create_a_database() {
    for flag in ["--help", "--version"] {
        let temp = tempfile::tempdir().unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"))
            .arg(flag)
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("project-memory-mcp"));
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    }
}

#[test]
fn rejects_relative_database_and_unknown_arguments() {
    for args in [vec!["--db", "relative.sqlite"], vec!["--surprise"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"))
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}
