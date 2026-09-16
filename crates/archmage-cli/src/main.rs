mod args;
mod output;
use clap::Parser;

#[tokio::main]
async fn main() {
    if let Err(error) = execute(args::Args::parse()).await {
        std::process::exit(output::error(&error));
    }
}

async fn execute(args: args::Args) -> archmage_core::error::Result<()> {
    use archmage_core::{config::Config, error::Error, provider::codex};
    match args.command {
        args::Commands::Doctor { offline } => {
            let tools = codex::inspect().await?;
            let root = std::env::var_os("ARCHMAGE_HOME").map(std::path::PathBuf::from);
            let config = Config::resolve(root.as_deref(), None)?;
            config.prepare()?;
            if !offline {
                return Err(Error::NotImplemented("online authentication probe"));
            }
            println!(
                "{}",
                serde_json::json!({
                    "tools": tools, "config": config,
                    "authentication": "not_verified_offline",
                    "database": "not_implemented_batch_b"
                })
            );
            Ok(())
        }
        _ => Err(Error::NotImplemented("workflow command")),
    }
}
