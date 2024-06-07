#![allow(dead_code)]

mod debug_app;
mod run_component_wasm;
mod to_view_model;
mod utils;
mod write_app_state;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "slipway")]
#[command(about = "Slipway CLI", long_about = None)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    #[command(arg_required_else_help = true)]
    Debug { input: PathBuf },
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Debug { input } => {
            debug_app::debug_app(input)?;
        }
    }

    Ok(())
}
