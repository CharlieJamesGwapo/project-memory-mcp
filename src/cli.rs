use clap::{Args, Subcommand};
use project_memory_mcp::{context_markdown, store::Store};
use serde_json::{Value, json};
use std::{path::Path, process::ExitCode};

#[derive(Subcommand)]
pub enum Command {
    /// Explicitly initialize an empty archive; preserve an existing supported archive.
    /// Show effective local capabilities without opening or creating an archive.
    Policy,
    /// Inspect credential patterns in one project, returning categories and IDs only.
    InspectSecrets {
        #[arg(long = "project", value_name = "UUID")]
        project_id: String,
        #[command(flatten)]
        page: PageArgs,
    },
    Init {
        #[arg(long)]
        json: bool,
    },
    /// Check an existing archive without starting MCP or changing memory records.
    Doctor {
        /// Machine-readable report. Exit codes: ready 0, missing 2, error 1.
        #[arg(long)]
        json: bool,
    },
    /// Print a JSON stdio configuration without changing client settings.
    PrintConfig,
    /// List project UUIDs as JSON, opening the archive read-only.
    Projects(PageArgs),
    /// Search literal text across project records; returns JSON previews.
    Search {
        #[arg(long = "project", value_name = "UUID")]
        project_id: String,
        query: String,
        #[arg(long, value_parser = ["note", "decision", "handoff"])]
        kind: Option<String>,
        #[arg(long)]
        include_superseded: bool,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Read one complete record as JSON, scoped to its project UUID.
    Record {
        #[arg(long = "project", value_name = "UUID")]
        project_id: String,
        #[arg(long = "record", value_name = "UUID")]
        record_id: String,
    },
    /// Build a full-record Markdown context brief; use --json for structured output.
    Context {
        #[arg(long = "project", value_name = "UUID")]
        project_id: String,
        #[arg(long, conflicts_with = "record_ids")]
        query: Option<String>,
        /// Terms is explicit lexical ranking, requires --query; literal is the default.
        #[arg(long, default_value = "literal", value_parser = ["literal", "terms"])]
        query_mode: String,
        /// Repeat to select complete records atomically, in the requested order.
        #[arg(long = "record", value_name = "UUID")]
        record_ids: Vec<String>,
        /// Canonical MCP-result byte cap, not the rendered CLI output size.
        #[arg(long, default_value_t = 16000)]
        max_bytes: usize,
        #[arg(long, default_value_t = 8)]
        max_records: usize,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Args)]
pub struct PageArgs {
    #[arg(long, default_value_t = 20)]
    limit: u32,
    #[arg(long)]
    cursor: Option<String>,
}

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("JSON value serializes")
    );
}

pub fn execute(
    command: Command,
    path: &Path,
    read_only: bool,
    policy: project_memory_mcp::security::AccessPolicy,
    policy_path: Option<&Path>,
) -> Result<ExitCode, String> {
    match command {
        Command::Policy => {
            print_json(&policy.summary(read_only));
            return Ok(ExitCode::SUCCESS);
        }
        Command::Doctor { json } => return Ok(doctor(path, json, policy)),
        Command::PrintConfig => {
            let binary = std::env::current_exe().map_err(|_| "Cannot locate this executable")?;
            let mut args = vec!["--db".to_string(), path.to_string_lossy().into_owned()];
            if let Some(policy_path) = policy_path {
                args.extend([
                    "--policy".into(),
                    policy_path.to_string_lossy().into_owned(),
                ]);
            }
            if read_only {
                args.push("--read-only".into());
            }
            print_json(&json!({"mcpServers":{"project-memory":{"command":binary,"args":args}}}));
            return Ok(ExitCode::SUCCESS);
        }
        Command::Init { json: as_json } => {
            if read_only {
                return Err(
                    "init requires writable mode; remove --read-only to initialize an archive"
                        .into(),
                );
            }
            let store =
                Store::open_with_policy(path, false, policy).map_err(|error| error.to_string())?;
            let diagnostics = store.diagnostics().map_err(|error| error.to_string())?;
            let report = json!({"status":"ready","database":path,"diagnostics":diagnostics,"message":"Archive is ready. Existing supported project records are preserved; no sample records were added.","next_action":"Run projects to inspect IDs, or print-config to connect your MCP client. Studio can now open this archive."});
            if as_json {
                print_json(&report);
            } else {
                println!("Project Memory archive is ready: {}", path.display());
                println!(
                    "No sample records were added. Run projects, print-config, or open the archive in Studio."
                );
            }
            return Ok(ExitCode::SUCCESS);
        }
        _ => {}
    }
    // Every inspection command opens read-only even without --read-only. Never
    // create an archive as a side effect of a typo, search, or context export.
    let reject_context = policy.secrets == project_memory_mcp::security::SecretMode::Reject;
    let mut store = Store::open_with_policy(path, true, policy).map_err(|e| e.to_string())?;
    let (operation, arguments, markdown) = match command {
        Command::InspectSecrets { project_id, page } => (
            "inspect_secrets",
            json!({"project_id":project_id,"limit":page.limit,"cursor":page.cursor}),
            false,
        ),
        Command::Projects(page) => (
            "list_projects",
            json!({"limit":page.limit,"cursor":page.cursor}),
            false,
        ),
        Command::Search {
            project_id,
            query,
            kind,
            include_superseded,
            page,
        } => (
            "search_memory",
            json!({"project_id":project_id,"query":query,"kind":kind,"include_superseded":include_superseded,"limit":page.limit,"cursor":page.cursor}),
            false,
        ),
        Command::Record {
            project_id,
            record_id,
        } => (
            "get_record",
            json!({"project_id":project_id,"record_id":record_id}),
            false,
        ),
        Command::Context {
            project_id,
            query,
            query_mode,
            record_ids,
            max_bytes,
            max_records,
            json: as_json,
        } => {
            let mut arguments = json!({"project_id":project_id,"query_mode":query_mode,"max_bytes":max_bytes,"max_records":max_records});
            if let Some(query) = query {
                arguments["query"] = json!(query);
            }
            if !record_ids.is_empty() {
                arguments["record_ids"] = json!(record_ids);
            }
            ("get_context_bundle", arguments, !as_json)
        }
        _ => unreachable!("setup commands returned before opening a read-only store"),
    };
    let value = store
        .execute(operation, arguments)
        .map_err(|error| error.to_string())?;
    if operation == "get_context_bundle" && reject_context {
        project_memory_mcp::security::reject_secrets(&value).map_err(|e| e.to_string())?;
    }
    if markdown {
        print!("{}", context_markdown(&value));
    } else {
        print_json(&value);
    }
    Ok(ExitCode::SUCCESS)
}

fn doctor(
    path: &Path,
    as_json: bool,
    policy: project_memory_mcp::security::AccessPolicy,
) -> ExitCode {
    let missing =
        std::fs::symlink_metadata(path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound);
    let (report, code): (Value, u8) = if missing {
        (
            json!({"status":"missing","database":path,"message":"No Project Memory archive exists at this path.","next_action":"Run init explicitly to initialize an archive, or connect this binary through your MCP client in writable mode. Use --read-only only after an archive exists."}),
            2,
        )
    } else {
        match Store::open_with_policy(path, true, policy).and_then(|store| store.diagnostics()) {
            Ok(diagnostics) => (
                json!({"status":"ready","database":path,"message":"Archive is readable and its SQLite integrity check passed.","diagnostics":diagnostics,"next_action":"Run projects or context to inspect it, or print-config to see a JSON stdio example."}),
                0,
            ),
            Err(error) => (
                json!({"status":"error","database":path,"message":error.to_string(),"error_code":error.code(),"next_action":"Choose an existing Project Memory archive. Check file permissions and upgrade if its schema is newer. Doctor does not repair or modify memory records or schema; SQLite may maintain WAL/SHM coordination files."}),
                1,
            ),
        }
    };
    if as_json {
        print_json(&report);
    } else {
        println!(
            "Project Memory doctor: {}",
            report["status"].as_str().unwrap()
        );
        println!("Database: {path:?}");
        println!("{}", report["message"].as_str().unwrap());
        if let Some(diagnostics) = report.get("diagnostics") {
            println!(
                "Schema {} · {} projects · {} records · read-only inspection",
                diagnostics["schema_version"],
                diagnostics["project_count"],
                diagnostics["record_count"]
            );
        }
        println!("{}", report["next_action"].as_str().unwrap());
    }
    ExitCode::from(code)
}
