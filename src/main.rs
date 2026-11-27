use clap::Parser;
use project_memory_mcp::{paths::default_database_path, server::MemoryServer, store::Store};
use rmcp::{ServiceExt, transport::stdio};
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(
    name = "project-memory-mcp",
    version,
    about = "Local project memory and handoffs for AI assistants over MCP stdio"
)]
struct Args {
    /// Absolute SQLite database path. Defaults to the platform's user data directory.
    #[arg(long, env = "PROJECT_MEMORY_DB")]
    db: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    match run(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("project-memory-mcp: {message}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Result<(), String> {
    let path = match args.db {
        Some(path) => path,
        None => default_database_path().map_err(|error| error.to_string())?,
    };
    let store = Store::open(&path).map_err(|e| e.to_string())?;
    let service = MemoryServer::new(store).serve(stdio()).await.map_err(|_| {
        "MCP initialization failed; connect through an MCP stdio client".to_string()
    })?;
    tokio::select! {
        result = service.waiting() => { result.map_err(|_|"MCP session ended unexpectedly".to_string())?; }
        result = tokio::signal::ctrl_c() => { result.map_err(|_|"Cannot handle shutdown signal".to_string())?; }
    }
    Ok(())
}
