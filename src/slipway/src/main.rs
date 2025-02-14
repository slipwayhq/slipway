#![allow(dead_code)]

mod canvas;
mod component_runners;
mod debug_rig;
mod host_error;
mod package;
mod permissions;
mod render_state;
mod run_rig;
mod serve;
mod to_view_model;
mod utils;

#[cfg(test)]
mod test_utils;

use std::path::PathBuf;

use clap::{
    builder::{
        styling::{AnsiColor, Effects},
        Styles,
    },
    Args, Parser, Subcommand,
};
use permissions::CommonPermissionsArgs;
use time::{format_description, OffsetDateTime};
use tracing::Level;
use tracing_subscriber::{fmt::time::FormatTime, FmtSubscriber};

const WASM_INTERFACE_TYPE_STR: &str = include_str!("../../../wit/latest/slipway.wit");

#[derive(Debug, Parser)]
#[command(name = "slipway")]
#[command(about = "Slipway CLI", long_about = None)]
#[command(color = clap::ColorChoice::Auto)]
#[command(styles = get_styles())]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

fn get_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .usage(AnsiColor::Green.on_default() | Effects::BOLD)
        .literal(AnsiColor::Blue.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Green.on_default())
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// Run a Slipway rig.
    #[command(arg_required_else_help = true)]
    Run {
        /// The path to the rig file.
        rig: PathBuf,

        #[command(flatten)]
        common: CommonRunArgs,

        /// The optional folder path to save the rig outputs to.
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },

    /// Debug a Slipway rig.
    #[command(arg_required_else_help = true)]
    Debug {
        /// The path to the rig file.
        rig: PathBuf,

        #[command(flatten)]
        common: CommonRunArgs,
    },

    /// Debug a Slipway component.
    #[command(arg_required_else_help = true)]
    DebugComponent {
        /// The path to the component file.
        component: PathBuf,

        /// The optional path to the file containing the component's input.
        #[arg(short, long)]
        input: Option<PathBuf>,

        #[command(flatten)]
        common: CommonRunArgs,
    },

    /// Serve HTTP requests.
    #[command(arg_required_else_help = true)]
    Serve {
        /// The path to the server configuration files.
        path: PathBuf,
    },

    /// Package up a Slipway component into a .tar file.
    #[command(arg_required_else_help = true)]
    Package {
        /// The path to the directory containing the component files.
        folder_path: PathBuf,

        /// The log level (error, warn, info, debug, trace).
        #[arg(short, long)]
        log_level: Option<String>,
    },

    /// Output the WIT (WASM Interface Type) definition, for building Slipway components.
    #[command()]
    Wit,
}

#[derive(Debug, Args)]
struct CommonRunArgs {
    /// The log level (error, warn, info, debug, trace).
    #[arg(short, long)]
    log_level: Option<String>,

    /// The registry URL to interpolate and use in preference to the default registry.
    /// This can be specified multiple times to search multiple registries in order.
    /// For example:
    ///   https://registry.example.com/{publisher}/{name}/{version}
    ///   file:../slipway_{name}/artifacts/{publisher}.{name}.{version}.tar
    #[arg(short, long, verbatim_doc_comment)]
    registry_url: Vec<String>,

    #[command(flatten)]
    permissions: CommonPermissionsArgs,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    set_ctrl_c_handler();

    let use_multi_threaded = match args.command {
        Commands::Serve { .. } => true,
        _ => false,
    };

    if use_multi_threaded {
        let mtr = tokio::runtime::Builder::new_multi_thread().build()?;
        mtr.block_on(async {
            // Your async main logic goes here
            main_multi_threaded(args).await
        });
    } else {
        let str = tokio::runtime::Builder::new_current_thread().build()?;
        str.block_on(async {
            // Your async main logic goes here
            main_single_threaded(args).await
        });
    };

    Ok(())
}

async fn main_single_threaded(args: Cli) -> anyhow::Result<()> {
    set_ctrl_c_handler();

    match args.command {
        Commands::Debug { rig, common } => {
            let log_level = common.log_level;
            let registry_url = common.registry_url;
            configure_tracing(log_level);
            let permissions = common.permissions.into_permissions()?;
            debug_rig::debug_rig_from_rig_file(
                &mut std::io::stdout(),
                rig,
                (&permissions).into(),
                registry_url,
            )
            .await?;
        }
        Commands::DebugComponent {
            component,
            input,
            common,
        } => {
            let log_level = common.log_level;
            let registry_url = common.registry_url;
            configure_tracing(log_level);
            let permissions = common.permissions.into_permissions()?;
            debug_rig::debug_rig_from_component_file(
                &mut std::io::stdout(),
                component,
                input,
                (&permissions).into(),
                registry_url,
            )
            .await?;
        }
        Commands::Run {
            rig,
            common,
            output,
        } => {
            let log_level = common.log_level;
            let registry_url = common.registry_url;
            configure_tracing(log_level);
            let permissions = common.permissions.into_permissions()?;
            run_rig::run_rig(
                &mut std::io::stdout(),
                rig,
                (&permissions).into(),
                registry_url,
                output,
            )
            .await?;
        }
        Commands::Package {
            folder_path,
            log_level,
        } => {
            configure_tracing(log_level);
            package::package_component(&folder_path)?;
        }
        Commands::Wit => {
            println!("{}", WASM_INTERFACE_TYPE_STR);
        }
        Commands::Serve { path: _ } => {
            panic!("Serve command is not supported in single-threaded mode.");
        }
    }

    Ok(())
}

async fn main_multi_threaded(args: Cli) -> anyhow::Result<()> {
    set_ctrl_c_handler();

    match args.command {
        Commands::Serve { path } => {
            serve::serve(path).await?;
        }
        _ => {
            panic!("Command is not supported in multi-threaded mode.");
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

struct CustomTimer;

impl FormatTime for CustomTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let now = OffsetDateTime::now_utc();
        let format = format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
            .expect("Timestamp format should be valid");
        write!(w, "{}", now.format(&format).unwrap())
    }
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

    let subscriber = FmtSubscriber::builder()
        .with_target(false)
        .with_timer(CustomTimer)
        .with_max_level(log_level)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
