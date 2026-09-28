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
    use archmage_core::{
        config::Config,
        error::Error,
        provider::codex,
        runtime::{CheckSpec, RunRequest, Runtime},
    };
    match args.command {
        args::Commands::Internal { command } => match command {
            args::InternalCommands::Check {
                endpoint,
                attempt,
                check,
            } => {
                #[cfg(unix)]
                {
                    let evidence =
                        archmage_core::check_runner::request_check(&endpoint, &attempt, &check)
                            .await?;
                    println!(
                        "{}",
                        serde_json::to_string(&evidence)
                            .map_err(|e| Error::Preflight(e.to_string()))?
                    );
                    Ok(())
                }
                #[cfg(not(unix))]
                Err(Error::Preflight(
                    "internal check helper requires Unix sockets".into(),
                ))
            }
        },
        args::Commands::Doctor { offline } => {
            let tools = codex::inspect().await?;
            let root = std::env::var_os("ARCHMAGE_HOME").map(std::path::PathBuf::from);
            let config = Config::resolve(root.as_deref(), None)?;
            config.prepare()?;
            let database =
                archmage_core::store::Store::open(&config.data_root.join("archmage.db"))?
                    .health()?;
            let probe = if offline {
                None
            } else {
                eprintln!(
                    "Invoking real Codex (read-only, 120s deadline); uses your configured model and account."
                );
                Some(codex::readonly_probe_logged(&tools, &config.data_root).await?)
            };
            println!(
                "{}",
                serde_json::json!({
                    "tools": tools, "config": config,
                    "authentication": if offline { "not_verified_offline" } else { "verified" },
                    "probe": probe,
                    "database": database
                })
            );
            Ok(())
        }
        args::Commands::Run { repo, goal, check } => {
            let root = std::env::var_os("ARCHMAGE_HOME").map(std::path::PathBuf::from);
            let runtime = Runtime::open(Config::resolve(root.as_deref(), Some(&repo))?)?;
            let checks = check
                .into_iter()
                .enumerate()
                .map(|(index, command)| CheckSpec::new(format!("check-{}", index + 1), command))
                .collect::<archmage_core::error::Result<Vec<_>>>()?;
            let request = RunRequest::new(repo, goal, checks)?;
            let run_id = runtime.start_run(&request).await?;
            eprintln!("Run {run_id} started with real Codex.");
            let snapshot = runtime.run_to_terminal(&run_id).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .map_err(|error| Error::Preflight(error.to_string()))?
            );
            Ok(())
        }
        args::Commands::Status { run_id, json } => {
            let runtime = open_runtime()?;
            let snapshot = runtime.snapshot(&run_id)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot)
                        .map_err(|error| Error::Preflight(error.to_string()))?
                );
            } else {
                println!(
                    "{}  {}  {}",
                    snapshot.id,
                    snapshot.state,
                    snapshot.outcome.as_deref().unwrap_or("-")
                );
                for node in snapshot.nodes {
                    println!("  {:<10} {}", node.key, node.state);
                }
            }
            Ok(())
        }
        args::Commands::Logs { run_id, follow } => {
            if follow {
                return Err(Error::NotImplemented("logs --follow"));
            }
            for event in open_runtime()?.events(&run_id, 0)? {
                println!(
                    "{}",
                    serde_json::to_string(&event)
                        .map_err(|error| Error::Preflight(error.to_string()))?
                );
            }
            Ok(())
        }
        args::Commands::Resume { .. } => Err(Error::NotImplemented("resume")),
        args::Commands::Cancel { .. } => Err(Error::NotImplemented("cancel")),
    }
}

fn open_runtime() -> archmage_core::error::Result<archmage_core::runtime::Runtime> {
    let root = std::env::var_os("ARCHMAGE_HOME").map(std::path::PathBuf::from);
    archmage_core::runtime::Runtime::open(archmage_core::config::Config::resolve(
        root.as_deref(),
        None,
    )?)
}
