use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "snipx")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Check,
    Resolve,
    Export,
    Fmt,
}

fn main() {
    let _cli = Cli::parse();
}
