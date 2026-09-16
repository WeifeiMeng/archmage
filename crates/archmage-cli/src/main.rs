mod args;
mod output;
use clap::Parser;

fn main() {
    let _args = args::Args::parse();
    std::process::exit(output::error(&archmage_core::error::Error::NotImplemented(
        "command",
    )));
}
