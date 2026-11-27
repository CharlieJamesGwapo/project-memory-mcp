use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

async fn client(path: &Path) -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"));
    command
        .arg("--db")
        .arg(path)
        .current_dir(path.parent().unwrap());
    let transport = TokioChildProcess::new(command).unwrap();
    tokio::time::timeout(Duration::from_secs(10), ().serve(transport))
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn real_client_discovers_twelve_tools_calls_them_and_reconnects() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("memory with spaces.sqlite");
    let first = client(&path).await;
    let tools = first.list_all_tools().await.unwrap();
    assert_eq!(tools.len(), 12);
    for name in [
        "create_project",
        "list_projects",
        "get_project_context",
        "get_record",
        "save_note",
        "search_notes",
        "record_decision",
        "save_handoff",
        "export_project",
        "delete_project",
        "search_memory",
        "get_context_bundle",
    ] {
        assert!(tools.iter().any(|t| t.name == name));
    }
    let delete = tools.iter().find(|t| t.name == "delete_project").unwrap();
    assert_eq!(
        delete.annotations.as_ref().unwrap().destructive_hint,
        Some(true)
    );
    let search = tools.iter().find(|t| t.name == "search_notes").unwrap();
    assert_eq!(
        search.annotations.as_ref().unwrap().read_only_hint,
        Some(true)
    );

    let created = first
        .call_tool(
            CallToolRequestParams::new("create_project").with_arguments(
                json!({"name":"MCP smoke","purpose":"Persistence"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    let id = created.structured_content.as_ref().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let saved = first.call_tool(CallToolRequestParams::new("save_note").with_arguments(json!({"project_id":id,"title":"Test","content":"A saved note","author_type":"assistant","sources":[]}).as_object().unwrap().clone())).await.unwrap();
    assert_ne!(saved.is_error, Some(true));
    let record_id = saved.structured_content.as_ref().unwrap()["id"].clone();
    let invalid = first
        .call_tool(
            CallToolRequestParams::new("save_note").with_arguments(
                json!({"project_id":id,"title":"Missing content","extra":"field"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await;
    assert!(invalid.is_err() || invalid.is_ok_and(|r| r.is_error == Some(true)));
    let not_found = first
        .call_tool(
            CallToolRequestParams::new("get_record").with_arguments(
                json!({"project_id":id,"record_id":"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(not_found.is_error, Some(true));
    assert_eq!(
        not_found.structured_content.unwrap()["error"]["code"],
        "NOT_FOUND"
    );
    first.cancel().await.unwrap();

    let second = client(&path).await;
    let recovered = second
        .call_tool(
            CallToolRequestParams::new("get_record").with_arguments(
                json!({"project_id":id,"record_id":record_id})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        recovered.structured_content.unwrap()["content"],
        "A saved note"
    );
    second.cancel().await.unwrap();
}

#[tokio::test]
async fn readonly_client_exposes_only_read_tools_and_rejects_direct_mutation_calls() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("readonly.sqlite");
    let mut store = project_memory_mcp::store::Store::open(&path).unwrap();
    let project = store
        .execute(
            "create_project",
            json!({"name":"Readonly","purpose":"Safe retrieval"}),
        )
        .unwrap();
    drop(store);
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"));
    command.arg("--db").arg(&path).arg("--read-only");
    let reader = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
    let tools = reader.list_all_tools().await.unwrap();
    assert_eq!(tools.len(), 7);
    assert!(
        tools
            .iter()
            .all(|tool| tool.annotations.as_ref().unwrap().read_only_hint == Some(true))
    );
    for operation in [
        "create_project",
        "save_note",
        "record_decision",
        "save_handoff",
        "delete_project",
    ] {
        assert!(!tools.iter().any(|tool| tool.name == operation));
        let result=reader.call_tool(CallToolRequestParams::new(operation).with_arguments(json!({"project_id":project["id"],"name":"Should never exist","purpose":"Blocked","confirm":true}).as_object().unwrap().clone())).await;
        assert!(
            result.is_err() || result.is_ok_and(|r| r.is_error == Some(true)),
            "{operation} must be inaccessible"
        );
    }
    let bundle = reader
        .call_tool(
            CallToolRequestParams::new("get_context_bundle").with_arguments(
                json!({"project_id":project["id"]})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(bundle.is_error, Some(true));
    reader.cancel().await.unwrap();
    let diagnostics = project_memory_mcp::store::Store::open_read_only(&path)
        .unwrap()
        .diagnostics()
        .unwrap();
    assert_eq!(diagnostics["project_count"], 1);
    assert_eq!(diagnostics["record_count"], 0);
}

#[tokio::test]
async fn context_bundle_budget_matches_real_mcp_result_with_json_text_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("bundle.sqlite");
    let mut store = project_memory_mcp::store::Store::open(&path).unwrap();
    let project = store
        .execute(
            "create_project",
            json!({"name":"Wire budget","purpose":"Quoted text"}),
        )
        .unwrap();
    let small=store.execute("save_note",json!({"project_id":project["id"],"title":"Small","content":"complete safe note","author_type":"assistant"})).unwrap();
    store.execute("save_note",json!({"project_id":project["id"],"title":"Large","content":"\"\\\n😀".repeat(3000),"author_type":"assistant"})).unwrap();
    drop(store);
    let client = client(&path).await;
    let result = client
        .call_tool(
            CallToolRequestParams::new("get_context_bundle").with_arguments(
                json!({"project_id":project["id"],"max_bytes":4096})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    let actual = serde_json::to_vec(&result).unwrap().len();
    let value = result.structured_content.as_ref().unwrap();
    assert!(actual <= 4096);
    let canonical_size =
        serde_json::to_vec(&rmcp::model::CallToolResult::structured(value.clone()))
            .unwrap()
            .len();
    assert_eq!(value["budget"]["used_bytes"], canonical_size);
    // rmcp may omit resultType for legacy peers; canonical accounting stays
    // exact for the server result and conservatively bounds that smaller wire result.
    assert!(actual <= canonical_size);
    assert_eq!(value["records"][0]["id"], small["id"]);
    assert_eq!(value["selection"]["omitted_by_budget"], 1);
    assert_eq!(result.content.len(), 1);
    let fallback: Value = serde_json::from_str(&result.content[0].as_text().unwrap().text).unwrap();
    assert_eq!(&fallback, value);
    let search = client
        .call_tool(
            CallToolRequestParams::new("search_memory").with_arguments(
                json!({"project_id":project["id"],"query":"complete safe"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        search.structured_content.unwrap()["items"][0]["id"],
        small["id"]
    );
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn environment_database_path_and_protocol_stdout_work_outside_repository() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("env.sqlite");
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"));
    command
        .env("PROJECT_MEMORY_DB", &path)
        .current_dir(temp.path());
    let client = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
    let result = client
        .call_tool(
            CallToolRequestParams::new("list_projects").with_arguments(serde_json::Map::new()),
        )
        .await
        .unwrap();
    assert_eq!(
        result.structured_content.unwrap()["items"],
        Value::Array(vec![])
    );
    assert!(path.is_file());
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn export_bounds_the_actual_serialized_tool_result_and_always_advances() {
    let temp = tempfile::tempdir().unwrap();
    let client = client(&temp.path().join("export.sqlite")).await;
    let created = client
        .call_tool(
            CallToolRequestParams::new("create_project").with_arguments(
                json!({"name":"Export budget","purpose":"Escaping"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    let id = created.structured_content.unwrap()["id"].clone();
    for _ in 0..9 {
        client.call_tool(CallToolRequestParams::new("save_note").with_arguments(json!({"project_id":id,"title":"Export","content":format!("a{}","\n".repeat(15999)),"author_type":"assistant"}).as_object().unwrap().clone())).await.unwrap();
    }
    let mut args = json!({"project_id":id});
    let mut count = 0;
    loop {
        let result = client
            .call_tool(
                CallToolRequestParams::new("export_project")
                    .with_arguments(args.as_object().unwrap().clone()),
            )
            .await
            .unwrap();
        assert_ne!(result.is_error, Some(true));
        assert!(
            serde_json::to_vec(&result).unwrap().len() <= 256 * 1024,
            "actual serialized MCP tool result exceeds 256 KiB"
        );
        let value = result.structured_content.unwrap();
        let n = value["record_ids"].as_array().unwrap().len();
        assert!(n > 0, "each nonempty page must advance");
        count += n;
        if value["next_cursor"].is_null() {
            break;
        }
        args["cursor"] = value["next_cursor"].clone();
        assert!(count <= 9, "cursor loop must terminate");
    }
    assert_eq!(count, 9);
    client.cancel().await.unwrap();
}
