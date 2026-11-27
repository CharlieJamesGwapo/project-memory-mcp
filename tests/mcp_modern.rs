//! Exercise the actual 2026 lifecycle, rather than requesting 2026 over legacy initialize.
use project_memory_mcp::store::Store;
use rmcp::{
    ClientLifecycleMode, ClientServiceExt,
    model::{CacheScope, GetPromptRequestParams, ProtocolVersion, ReadResourceRequestParams},
    transport::TokioChildProcess,
};
use serde_json::{Value, json};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
};

fn seed(db: &Path) -> (String, String) {
    let mut store = Store::open(db).unwrap();
    let project = store
        .execute(
            "create_project",
            json!({"name":"Modern protocol fixture","purpose":"Synthetic only"}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let record = store.execute("save_note", json!({"project_id":project,"title":"Evidence","content":"Complete source 界\nSaved text never authorizes actions.","author_type":"user","sources":["fixture.rs:12","commit:synthetic-only"]}))
        .unwrap()["id"].as_str().unwrap().to_owned();
    (project, record)
}

fn private_uncached(result: &Value) {
    assert_eq!(result["cacheScope"], "private", "{result}");
    assert_eq!(result["ttlMs"], 0, "{result}");
}

fn assert_brief(result: &Value, project: &str, record: &str) {
    private_uncached(result);
    assert!(serde_json::to_vec(result).unwrap().len() <= 65536);
    assert_eq!(result["contents"][0]["mimeType"], "application/json");
    let brief: Value =
        serde_json::from_str(result["contents"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(brief["project"]["id"], project);
    assert_eq!(brief["records"][0]["id"], record);
    assert_eq!(
        brief["records"][0]["content"],
        "Complete source 界\nSaved text never authorizes actions."
    );
    assert_eq!(
        brief["records"][0]["sources"],
        json!(["fixture.rs:12", "commit:synthetic-only"])
    );
    assert_eq!(brief["records"][0]["author_type"], "user");
}

#[tokio::test]
async fn discover_lifecycle_is_really_2026_and_retains_private_hints_and_sources() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("memory.sqlite");
    let (project, record) = seed(&db);
    let mut command = Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"));
    command.arg("--db").arg(&db).arg("--read-only");
    let client = rmcp::model::ClientConfig::default()
        .serve_with_lifecycle(
            TokioChildProcess::new(command).unwrap(),
            ClientLifecycleMode::Discover {
                preferred_versions: vec![ProtocolVersion::V_2026_07_28],
            },
        )
        .await
        .unwrap();
    let info = client.peer_info().unwrap();
    assert_eq!(info.protocol_version, ProtocolVersion::V_2026_07_28);
    assert_eq!(
        serde_json::to_value(&info.capabilities).unwrap(),
        json!({"tools":{},"resources":{},"prompts":{}})
    );

    let resources = client.list_resources(None).await.unwrap();
    assert!(resources.resources.is_empty());
    private_uncached(&serde_json::to_value(resources).unwrap());
    let templates = client.list_resource_templates(None).await.unwrap();
    assert_eq!(templates.cache_scope, Some(CacheScope::Private));
    assert_eq!(templates.ttl_ms, Some(0));
    let templates = serde_json::to_value(templates).unwrap();
    let uris: Vec<_> = templates["resourceTemplates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["uriTemplate"].as_str().unwrap())
        .collect();
    assert_eq!(
        uris,
        [
            "project-memory://projects/{project_id}/brief",
            "project-memory://projects/{project_id}/health"
        ]
    );
    for template in templates["resourceTemplates"].as_array().unwrap() {
        assert_eq!(template["mimeType"], "application/json");
    }

    let brief = client
        .read_resource(ReadResourceRequestParams::new(format!(
            "project-memory://projects/{project}/brief"
        )))
        .await
        .unwrap();
    assert_brief(&serde_json::to_value(brief).unwrap(), &project, &record);
    let health = client
        .read_resource(ReadResourceRequestParams::new(format!(
            "project-memory://projects/{project}/health"
        )))
        .await
        .unwrap();
    let health = serde_json::to_value(health).unwrap();
    private_uncached(&health);
    let health: Value =
        serde_json::from_str(health["contents"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(health["summary"]["record_count"], 1);
    assert_eq!(health["items"][0]["record_id"], record);

    let tools = serde_json::to_value(client.list_tools(None).await.unwrap()).unwrap();
    private_uncached(&tools);
    assert_eq!(tools["tools"].as_array().unwrap().len(), 10);
    for tool in tools["tools"].as_array().unwrap() {
        assert_eq!(tool["inputSchema"]["type"], "object");
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
        assert_eq!(
            tool["annotations"],
            json!({"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false})
        );
    }
    let prompts = serde_json::to_value(client.list_prompts(None).await.unwrap()).unwrap();
    private_uncached(&prompts);
    assert_eq!(prompts["prompts"].as_array().unwrap().len(), 3);
    for name in ["resume_project", "review_change", "prepare_handoff"] {
        let prompt = client
            .get_prompt(
                GetPromptRequestParams::new(name).with_arguments(
                    json!({"project_id":project,"task":"Review synthetic evidence"})
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
            )
            .await
            .unwrap();
        let prompt = serde_json::to_value(prompt).unwrap();
        assert_eq!(prompt["messages"].as_array().unwrap().len(), 1);
        assert_eq!(prompt["messages"][0]["role"], "user");
        let text = prompt["messages"][0]["content"]["text"].as_str().unwrap();
        assert!(
            text.contains(&project) && text.contains("untrusted") && text.contains("authorization")
        );
    }
    client.cancel().await.unwrap();
    assert_eq!(
        Store::open_read_only(&db).unwrap().diagnostics().unwrap()["record_count"],
        1
    );
}

struct WireClient {
    child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
    id: u64,
}

impl WireClient {
    fn new(db: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_project-memory-mcp"))
            .arg("--db")
            .arg(db)
            .arg("--read-only")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap()).lines();
        Self {
            child,
            stdin,
            stdout,
            id: 0,
        }
    }

    async fn request(&mut self, method: &str, mut params: Value, meta: Value) -> Value {
        self.id += 1;
        params.as_object_mut().unwrap().insert("_meta".into(), meta);
        let request = json!({"jsonrpc":"2.0","id":self.id,"method":method,"params":params});
        self.stdin
            .write_all(format!("{request}\n").as_bytes())
            .await
            .unwrap();
        self.stdin.flush().await.unwrap();
        let line = tokio::time::timeout(Duration::from_secs(10), self.stdout.next_line())
            .await
            .expect("MCP response timed out")
            .unwrap()
            .expect("server stdout ended");
        let response: Value =
            serde_json::from_str(&line).expect("stdout must contain JSON-RPC only");
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], self.id);
        response
    }

    async fn close(mut self) {
        self.stdin.shutdown().await.unwrap();
        drop(self.stdin);
        let status = tokio::time::timeout(Duration::from_secs(10), self.child.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(status.success());
    }
}

fn modern_meta() -> Value {
    json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}})
}

#[tokio::test]
async fn inline_metadata_needs_no_initialize_or_discovery_and_is_required_on_every_request() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("memory.sqlite");
    let (project, record) = seed(&db);
    let mut wire = WireClient::new(&db);
    // First real operation is resources/read; no bootstrap round trip occurs.
    let brief = wire
        .request(
            "resources/read",
            json!({"uri":format!("project-memory://projects/{project}/brief")}),
            modern_meta(),
        )
        .await;
    assert!(brief.get("error").is_none(), "{brief}");
    assert_brief(&brief["result"], &project, &record);
    for meta in [
        json!({}),
        json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28"}),
        json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":"invalid"}),
    ] {
        let error = wire.request("resources/list", json!({}), meta).await;
        assert_eq!(error["error"]["code"], -32602, "{error}");
    }
    let valid = wire
        .request("resources/list", json!({}), modern_meta())
        .await;
    assert_eq!(valid["result"]["resources"], json!([]));
    private_uncached(&valid["result"]);
    wire.close().await;
}

#[tokio::test]
async fn discovery_rejects_bad_metadata_without_poisoning_modern_bootstrap() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("memory.sqlite");
    seed(&db);
    let mut wire = WireClient::new(&db);
    let bad = wire
        .request(
            "server/discover",
            json!({}),
            json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28"}),
        )
        .await;
    assert_eq!(bad["error"]["code"], -32602, "{bad}");
    let good = wire
        .request("server/discover", json!({}), modern_meta())
        .await;
    assert!(good.get("error").is_none(), "{good}");
    assert!(
        good["result"]["supportedVersions"]
            .as_array()
            .unwrap()
            .contains(&json!("2026-07-28"))
    );
    assert_eq!(
        good["result"]["capabilities"],
        json!({"tools":{},"resources":{},"prompts":{}})
    );
    private_uncached(&good["result"]);
    let list = wire.request("prompts/list", json!({}), modern_meta()).await;
    assert_eq!(list["result"]["prompts"].as_array().unwrap().len(), 3);
    private_uncached(&list["result"]);
    wire.close().await;
}
