mod args;
mod commands;
mod report;

use clap::Parser;

use args::{Cli, Command};

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Preview(args) => commands::preview::run(&args),
    }
}
