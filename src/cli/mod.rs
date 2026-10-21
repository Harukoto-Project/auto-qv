mod args;
mod check_report;
mod commands;
mod prepared;
mod progress;
mod report;
mod stroke_data;
mod text_input;

use clap::Parser;

use args::{Cli, Command};

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Write(args) => commands::write::run(&args),
        Command::Preview(args) => commands::preview::run(&args),
        Command::Check(args) => commands::check::run(&args),
    }
}
