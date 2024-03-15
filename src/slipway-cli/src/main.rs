#![allow(dead_code)]

mod cli;
mod debug_app;
mod to_view_model;
mod utils;
mod write_app_state;

use clap::Parser;
use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Debug { input } => {
            debug_app::debug_app(input)?;
        }
    }

    Ok(())
}
