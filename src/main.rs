#![forbid(unsafe_code)]

use clap::Parser;
use project_memory_mcp::{paths::default_database_path, server::MemoryServer, store::Store};
use rmcp::{ServiceExt, transport::stdio};
use std::{path::PathBuf, process::ExitCode};

mod cli;

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
    command: Option<cli::Command>,
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
    if let Some(command) = args.command {
        return cli::execute(command, &path, args.read_only);
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
