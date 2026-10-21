mod cli;
mod config;
mod dataset;
mod error;
mod glyph;
mod input;
mod layout;
mod missing_log;
mod planner;
mod runner;

fn main() -> anyhow::Result<()> {
    cli::run()
}
