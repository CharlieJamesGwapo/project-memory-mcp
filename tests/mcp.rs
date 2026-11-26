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
async fn real_client_discovers_ten_tools_calls_them_and_reconnects() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("memory with spaces.sqlite");
    let first = client(&path).await;
    let tools = first.list_all_tools().await.unwrap();
    assert_eq!(tools.len(), 10);
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
