use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "archmage",
    version,
    about = "Local capability-agent collaboration"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Check local prerequisites; online mode will invoke real Codex.
    Doctor {
        #[arg(long)]
        offline: bool,
    },
    /// Execute the four-capability workflow with real local Codex.
    Run {
        #[arg(long)]
        repo: PathBuf,
        #[arg(long, value_parser = nonblank)]
        goal: String,
        #[arg(long, required = true, value_parser = nonblank)]
        check: Vec<String>,
    },
    /// Inspect a persisted run.
    Status {
        run_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Read a persisted timeline.
    Logs {
        run_id: String,
        #[arg(long)]
        follow: bool,
    },
    /// Resume a persisted run.
    Resume { run_id: String },
    /// Request cancellation.
    Cancel { run_id: String },
    #[command(hide = true)]
    Internal {
        #[command(subcommand)]
        command: InternalCommands,
    },
}

#[derive(Subcommand)]
pub enum InternalCommands {
    Check {
        #[arg(long)]
        endpoint: PathBuf,
        #[arg(long)]
        attempt: String,
        #[arg(long)]
        check: String,
    },
}

fn nonblank(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        Err("value must not be blank".into())
    } else {
        Ok(value.to_owned())
    }
}
