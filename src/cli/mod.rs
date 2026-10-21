mod args;
mod commands;
mod prepared;
mod progress;
mod report;

use clap::Parser;

use args::{Cli, Command};

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Write(args) => commands::write::run(&args),
        Command::Preview(args) => commands::preview::run(&args),
    }
}
