#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use project_memory_mcp::{paths::default_database_path, server::MemoryServer, store::Store};
use rmcp::{ServiceExt, transport::stdio};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    name = "project-memory-mcp",
    version,
    about = "Local project memory and handoffs for AI assistants over MCP stdio"
)]
struct Args {
    /// Absolute SQLite database path. Defaults to the platform's user data directory.
    #[arg(long, env = "PROJECT_MEMORY_DB", global = true)]
    db: Option<PathBuf>,
    /// Open an existing archive and expose only read tools. Never creates a database.
    #[arg(long, global = true)]
    read_only: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Check an existing archive without writing to it or starting MCP.
    Doctor {
        /// Print a machine-readable report. Exit codes: ready 0, missing 2, error 1.
        #[arg(long)]
        json: bool,
    },
    /// Print a Claude Desktop/Cursor-style JSON stdio config; does not change files.
    PrintConfig,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    match run(args).await {
        Ok(code) => code,
        Err(message) => {
            eprintln!("project-memory-mcp: {message}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Result<ExitCode, String> {
    let path = match args.db {
        Some(path) => path,
        None => default_database_path().map_err(|error| error.to_string())?,
    };
    if !path.is_absolute() {
        return Err("Database path must be absolute. Choose an explicit --db path.".into());
    }
    match args.command {
        Some(Command::Doctor { json }) => return Ok(doctor(&path, json)),
        Some(Command::PrintConfig) => {
            let binary = std::env::current_exe().map_err(|_| "Cannot locate this executable")?;
            let mut config_args = vec!["--db".to_string(), path.to_string_lossy().into_owned()];
            if args.read_only {
                config_args.push("--read-only".into());
            }
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &json!({"mcpServers":{"project-memory":{"command":binary,"args":config_args}}})
                )
                .expect("config serializes")
            );
            return Ok(ExitCode::SUCCESS);
        }
        None => {}
    }
    let store = if args.read_only {
        Store::open_read_only(&path)
    } else {
        Store::open(&path)
    }
    .map_err(|e| e.to_string())?;
    let service = MemoryServer::new(store).serve(stdio()).await.map_err(|_| {
        "MCP initialization failed; connect through an MCP stdio client".to_string()
    })?;
    tokio::select! {
        result = service.waiting() => { result.map_err(|_|"MCP session ended unexpectedly".to_string())?; }
        result = tokio::signal::ctrl_c() => { result.map_err(|_|"Cannot handle shutdown signal".to_string())?; }
    }
    Ok(ExitCode::SUCCESS)
}

fn doctor(path: &Path, as_json: bool) -> ExitCode {
    let missing =
        std::fs::symlink_metadata(path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound);
    let (report, code): (Value, u8) = if missing {
        (
            json!({"status":"missing","database":path,"message":"No Project Memory archive exists at this path.","next_action":"Connect this binary through your MCP client in writable mode to create an archive, or choose an existing --db path. Use --read-only only after an archive exists."}),
            2,
        )
    } else {
        match Store::open_read_only(path).and_then(|store| store.diagnostics()) {
            Ok(diagnostics) => (
                json!({"status":"ready","database":path,"message":"Archive is readable and its SQLite integrity check passed.","diagnostics":diagnostics,"next_action":"Connect Project Memory in your MCP client. Run print-config to see a JSON configuration example."}),
                0,
            ),
            Err(error) => (
                json!({"status":"error","database":path,"message":error.to_string(),"error_code":error.code(),"next_action":"Choose an existing Project Memory archive. Check file permissions and upgrade if its schema is newer. Doctor does not repair or modify memory records or schema; SQLite may maintain WAL/SHM coordination files."}),
                1,
            ),
        }
    };
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("report serializes")
        );
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
