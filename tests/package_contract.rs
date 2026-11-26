use serde_json::Value;
use std::path::Path;

#[test]
fn publication_metadata_matches_the_crate_and_skills_are_distributed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let registry: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("server.json")).expect("registry metadata must exist"),
    )
    .unwrap();
    assert_eq!(registry["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        registry["name"],
        "io.github.charliejamesgwapo/project-memory"
    );
    assert_eq!(registry["packages"][0]["registryType"], "cargo");
    assert_eq!(
        registry["packages"][0]["identifier"],
        env!("CARGO_PKG_NAME")
    );
    assert_eq!(registry["packages"][0]["transport"]["type"], "stdio");
    let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
    assert!(readme.contains("mcp-name: io.github.charliejamesgwapo/project-memory"));
    for skill in ["resume-project", "record-decision", "prepare-handoff"] {
        assert!(root.join("skills").join(skill).join("SKILL.md").is_file());
    }
    assert!(root.join("LICENSE").is_file());
}
