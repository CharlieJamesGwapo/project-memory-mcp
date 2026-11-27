use project_memory_mcp::store::Store;
use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::json;

#[tokio::test]
async fn default_and_scoped_wire_inventory_match_store_enforcement() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("memory.sqlite");
    let start = |policy: Option<&std::path::Path>| {
        let mut c = tokio::process::Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"));
        c.arg("--db").arg(&db);
        if let Some(p) = policy {
            c.arg("--policy").arg(p);
        }
        TokioChildProcess::new(c).unwrap()
    };
    let client = ().serve(start(None)).await.unwrap();
    let tools = client.list_all_tools().await.unwrap();
    assert_eq!(tools.len(), 14);
    assert!(!tools.iter().any(|t| t.name == "delete_project"));
    assert!(
        client
            .call_tool(
                CallToolRequestParams::new("delete_project").with_arguments(
                    json!({"project_id":"00000000-0000-4000-8000-000000000001","confirm":true})
                        .as_object()
                        .unwrap()
                        .clone()
                )
            )
            .await
            .is_err()
    );
    let secret = format!("ghp_{}", "A".repeat(36));
    let result=client.call_tool(CallToolRequestParams::new("search_memory").with_arguments(json!({"project_id":"00000000-0000-4000-8000-000000000001","query":"test","kind":secret}).as_object().unwrap().clone())).await.unwrap();
    assert_eq!(result.is_error, Some(true));
    assert!(
        !serde_json::to_string(&result).unwrap().contains(&secret),
        "Malformed argument error reflected credential"
    );
    client.cancel().await.unwrap();
    let mut store = Store::open(&db).unwrap();
    let p = store
        .execute("create_project", json!({"name":"Scoped","purpose":"wire"}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let hidden = store
        .execute("create_project", json!({"name":"Hidden","purpose":"wire"}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    drop(store);
    let policy = temp.path().join("policy.json");
    std::fs::write(&policy,json!({"projects":[p],"operations":["list_projects","get_project_context","get_access_policy"]}).to_string()).unwrap();
    let client = ().serve(start(Some(&policy))).await.unwrap();
    let tools = client.list_all_tools().await.unwrap();
    assert_eq!(tools.len(), 3);
    let result = client
        .call_tool(
            CallToolRequestParams::new("get_project_context")
                .with_arguments(json!({"project_id":hidden}).as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(true));
    assert_eq!(
        result.structured_content.unwrap()["error"]["code"],
        "ACCESS_DENIED"
    );
    let result = client
        .call_tool(
            CallToolRequestParams::new("list_projects").with_arguments(serde_json::Map::new()),
        )
        .await
        .unwrap();
    assert_eq!(
        result.structured_content.unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    client.cancel().await.unwrap();
}
