#![allow(dead_code)]

mod debug_app;
mod run_component_wasm;
mod to_view_model;
mod utils;

#[cfg(test)]
mod test_utils;

mod write_app_state;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Debug, Parser)]
#[command(name = "slipway")]
#[command(about = "Slipway CLI", long_about = None)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// Run a Slipway app indefinitely.
    #[command(arg_required_else_help = true)]
    Launch { path: PathBuf },

    /// Run a Slipway app once.
    #[command(arg_required_else_help = true)]
    Run { path: PathBuf },

    /// Debug a Slipway app.
    #[command(arg_required_else_help = true)]
    Debug { path: PathBuf },

    /// Debug a Slipway component.
    #[command(arg_required_else_help = true)]
    DebugComponent { path: PathBuf },
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    configure_tracing();
    set_ctrl_c_handler();

    match args.command {
        Commands::Debug { path } => {
            debug_app::debug_app_from_app_file(&mut std::io::stdout(), path)?;
        }
        Commands::DebugComponent { path } => {
            debug_app::debug_app_from_component_file(&mut std::io::stdout(), path)?;
        }
        Commands::Launch { path: _ } => {
            todo!();
        }
        Commands::Run { path: _ } => {
            todo!();
        }
    }

    Ok(())
}

fn set_ctrl_c_handler() {
    ctrlc::set_handler(move || {
        std::process::exit(1);
    })
    .expect("Error setting Ctrl-C handler");
}

fn configure_tracing() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
