//! Local, explicit project memory for MCP clients.
#![forbid(unsafe_code)]
pub mod error;
mod export;
pub use export::context_markdown;
pub mod input;
mod output;
mod pagination;
pub mod paths;
pub mod review;
pub mod security;
pub mod server;
pub mod store;
