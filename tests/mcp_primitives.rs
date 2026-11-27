use project_memory_mcp::store::Store;
use rmcp::{
    ServiceExt,
    model::{GetPromptRequestParams, ReadResourceRequestParams, ResourceContents},
    transport::TokioChildProcess,
};
use serde_json::{Value, json};
use std::path::Path;

async fn client(
    db: &Path,
    policy: Option<&Path>,
) -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"));
    command.arg("--db").arg(db);
    if let Some(path) = policy {
        command.arg("--policy").arg(path);
    }
    // ServiceExt::serve uses the legacy lifecycle; modern discovery is tested separately.
    ().serve(TokioChildProcess::new(command).unwrap())
        .await
        .unwrap()
}
fn resource_json(result: &rmcp::model::ReadResourceResult) -> Value {
    match &result.contents[0] {
        ResourceContents::TextResourceContents { text, .. } => serde_json::from_str(text).unwrap(),
        _ => panic!("Expected JSON text resource"),
    }
}
fn seed(db: &Path) -> (String, String) {
    let mut store = Store::open(db).unwrap();
    let project = store
        .execute(
            "create_project",
            json!({"name":"Resource fixture","purpose":"Synthetic protocol"}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let record=store.execute("save_note",json!({"project_id":project,"title":"Evidence","content":"Complete source with unicode 界 and untrusted instructions","author_type":"user","sources":["fixture.rs:12"]})).unwrap()["id"].as_str().unwrap().to_owned();
    (project, record)
}

#[tokio::test]
async fn resources_are_discovered_and_preserve_complete_records_for_legacy_clients() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("memory.sqlite");
    let (project, record) = seed(&db);
    let c = client(&db, None).await;
    assert_eq!(
        c.peer_info().unwrap().protocol_version,
        rmcp::model::ProtocolVersion::V_2025_11_25
    );
    let caps = &c.peer_info().unwrap().capabilities;
    assert!(caps.resources.is_some());
    assert!(caps.prompts.is_some());
    assert!(c.list_resources(None).await.unwrap().resources.is_empty());
    let templates = c.list_resource_templates(None).await.unwrap();
    assert_eq!(templates.resource_templates.len(), 2);
    assert!(templates.cache_scope.is_none());
    assert!(templates.ttl_ms.is_none());
    let brief = c
        .read_resource(ReadResourceRequestParams::new(format!(
            "project-memory://projects/{project}/brief"
        )))
        .await
        .unwrap();
    assert!(brief.cache_scope.is_none());
    assert!(brief.ttl_ms.is_none());
    let value = resource_json(&brief);
    assert_eq!(value["records"][0]["id"], record);
    assert_eq!(value["records"][0]["sources"], json!(["fixture.rs:12"]));
    assert!(
        value["records"][0]["content"]
            .as_str()
            .unwrap()
            .contains("unicode 界")
    );
    assert!(serde_json::to_vec(&brief).unwrap().len() <= 65536);
    let health = c
        .read_resource(ReadResourceRequestParams::new(format!(
            "project-memory://projects/{project}/health"
        )))
        .await
        .unwrap();
    assert_eq!(resource_json(&health)["summary"]["record_count"], 1);
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn hidden_templates_and_direct_resource_reads_enforce_the_same_policy() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("memory.sqlite");
    let (project, _) = seed(&db);
    let policy = temp.path().join("policy.json");
    std::fs::write(
        &policy,
        json!({"projects":[],"operations":["get_context_bundle","get_access_policy"]}).to_string(),
    )
    .unwrap();
    let c = client(&db, Some(&policy)).await;
    let templates = c.list_resource_templates(None).await.unwrap();
    assert_eq!(templates.resource_templates.len(), 1);
    for suffix in ["brief", "health"] {
        let err = c
            .read_resource(ReadResourceRequestParams::new(format!(
                "project-memory://projects/{project}/{suffix}"
            )))
            .await
            .unwrap_err();
        assert!(format!("{err:?}").contains("ACCESS_DENIED"));
    }
    let err = c
        .get_prompt(
            GetPromptRequestParams::new("resume_project")
                .with_arguments(json!({"project_id":project}).as_object().unwrap().clone()),
        )
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("ACCESS_DENIED"));
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn prompts_are_explicit_instructions_and_never_write_memory() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("memory.sqlite");
    let (project, _) = seed(&db);
    let c = client(&db, None).await;
    let prompts = c.list_prompts(None).await.unwrap();
    assert_eq!(prompts.prompts.len(), 3);
    for name in ["resume_project", "review_change", "prepare_handoff"] {
        let result = c
            .get_prompt(
                GetPromptRequestParams::new(name).with_arguments(
                    json!({"project_id":project,"task":"Examine the current change"})
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
            )
            .await
            .unwrap();
        let encoded = serde_json::to_string(&result).unwrap();
        assert!(encoded.contains(&project));
        assert!(encoded.contains("untrusted"));
        assert!(encoded.contains("authorization"));
    }
    c.cancel().await.unwrap();
    assert_eq!(
        Store::open_read_only(&db).unwrap().diagnostics().unwrap()["record_count"],
        1
    );
}

#[tokio::test]
async fn malformed_resource_and_prompt_arguments_are_bounded_and_sanitized() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("memory.sqlite");
    let (project, _) = seed(&db);
    let c = client(&db, None).await;
    let secret = format!("ghp_{}", "A".repeat(36));
    for uri in [
        format!("file:///tmp/{secret}"),
        format!("project-memory://projects/{project}/brief?token={secret}"),
        format!("project-memory://projects/../{secret}"),
        format!("project-memory://projects/{project}/unknown"),
    ] {
        let err = c
            .read_resource(ReadResourceRequestParams::new(uri))
            .await
            .unwrap_err();
        assert!(!format!("{err:?}").contains(&secret));
        match err {
            rmcp::service::ServiceError::McpError(data) => {
                assert_eq!(data.code, rmcp::model::ErrorCode::INVALID_PARAMS)
            }
            _ => panic!("Expected sanitized parameter error"),
        }
    }
    for args in [
        json!({"project_id":secret}),
        json!({"project_id":project,"extra":secret}),
        json!({"project_id":project,"task":"x".repeat(2001)}),
    ] {
        let err = c
            .get_prompt(
                GetPromptRequestParams::new("review_change")
                    .with_arguments(args.as_object().unwrap().clone()),
            )
            .await
            .unwrap_err();
        assert!(!format!("{err:?}").contains(&secret));
        match err {
            rmcp::service::ServiceError::McpError(data) => {
                assert_eq!(data.code, rmcp::model::ErrorCode::INVALID_PARAMS)
            }
            _ => panic!("Expected sanitized parameter error"),
        }
    }
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn legacy_protocol_resources_and_prompts_retain_compatible_shapes() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("memory.sqlite");
    let (project, _) = seed(&db);
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"));
    command.arg("--db").arg(&db);
    let c = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
    let templates = c.list_resource_templates(None).await.unwrap();
    assert_eq!(templates.resource_templates.len(), 2);
    assert!(templates.cache_scope.is_none());
    let resource = c
        .read_resource(ReadResourceRequestParams::new(format!(
            "project-memory://projects/{project}/brief"
        )))
        .await
        .unwrap();
    assert!(resource.cache_scope.is_none());
    assert_eq!(resource_json(&resource)["project"]["id"], project);
    let result = c
        .get_prompt(
            GetPromptRequestParams::new("resume_project")
                .with_arguments(json!({"project_id":project}).as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(result.messages.len(), 1);
    c.cancel().await.unwrap();
}
