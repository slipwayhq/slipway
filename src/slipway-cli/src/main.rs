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
    Launch {
        path: PathBuf,

        #[arg(short, long)]
        log_level: Option<String>,
    },

    /// Run a Slipway app once.
    #[command(arg_required_else_help = true)]
    Run {
        path: PathBuf,

        #[arg(short, long)]
        log_level: Option<String>,
    },

    /// Debug a Slipway app.
    #[command(arg_required_else_help = true)]
    Debug {
        path: PathBuf,

        #[arg(short, long)]
        log_level: Option<String>,
    },

    /// Debug a Slipway component.
    #[command(arg_required_else_help = true)]
    DebugComponent {
        path: PathBuf,

        #[arg(short, long)]
        log_level: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    set_ctrl_c_handler();

    match args.command {
        Commands::Debug { path, log_level } => {
            configure_tracing(log_level);
            debug_app::debug_app_from_app_file(&mut std::io::stdout(), path)?;
        }
        Commands::DebugComponent { path, log_level } => {
            configure_tracing(log_level);
            debug_app::debug_app_from_component_file(&mut std::io::stdout(), path)?;
        }
        Commands::Launch { path: _, log_level } => {
            configure_tracing(log_level);
            todo!();
        }
        Commands::Run { path: _, log_level } => {
            configure_tracing(log_level);
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

fn configure_tracing(log_level: Option<String>) {
    let log_level = match log_level.map(|level| level.to_lowercase()).as_deref() {
        Some("error") => Level::ERROR,
        Some("warn") => Level::WARN,
        Some("info") => Level::INFO,
        Some("debug") => Level::DEBUG,
        Some("trace") => Level::TRACE,
        Some(_) => panic!("invalid log level. must be one of [error, warn, info, debug, trace]."),
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
