#![allow(dead_code)]

mod canvas;
mod component_runners;
mod debug_rig;
mod host_error;
mod package;
mod permissions;
mod primitives;
mod run_rig;
mod serve;

#[cfg(test)]
mod test_utils;

use std::path::PathBuf;

use clap::{
    Args, Parser, Subcommand,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};
use permissions::CommonPermissionsArgs;
use primitives::{ApiKeyName, DeviceName, PlaylistName, RigName};
use semver::Version;
use slipway_engine::{Name, Publisher};
use slipway_host::hash_string;
use time::{OffsetDateTime, format_description};
use tracing::{Level, info};
use tracing_subscriber::{FmtSubscriber, fmt::time::FormatTime};

const WASM_INTERFACE_TYPE_STR: &str = include_str!("../../../wit/latest/slipway.wit");
const SLIPWAY_COMPONENT_FILE_NAME: &str = "slipway_component.json";
const AOT_ARTIFACT_FOLDER_NAME: &str = "aot";

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

        /// The optional file path to save the flattened debug rig to.
        #[arg(short, long)]
        output_debug_rig: Option<std::path::PathBuf>,

        /// The optional folder path where additional fonts are located.
        #[arg(short, long)]
        fonts: Option<std::path::PathBuf>,
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

    /// Create default configuration for a component.
    #[command(arg_required_else_help = true)]
    InitComponent {
        /// The component publisher name (lowercase alphanumeric plus underscores).
        #[arg(short, long)]
        publisher: Publisher,

        /// The component name (lowercase alphanumeric plus underscores).
        #[arg(short, long)]
        name: Name,
    },

    /// Create default configuration for a rig.
    #[command(arg_required_else_help = true)]
    InitRig {
        /// The rig name (lowercase alphanumeric plus underscores).
        #[arg(short, long)]
        name: RigName,
    },

    /// Serve HTTP requests. Use `slipway serve --help` for more commands.
    #[command(arg_required_else_help = true)]
    Serve {
        /// The path to the server configuration files.
        path: PathBuf,

        /// Whether to enable using AOT compiled artifacts, generated
        /// by previously running `slipway serve <path> --aot-compile`.
        /// This should be used with caution. AOT compiled files
        /// must be compatible with the target machine architecture,
        /// and should be created with this exact version of Slipway.
        #[arg(long)]
        aot: bool,

        #[command(subcommand)]
        subcommand: Option<ServeCommands>,
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

    /// Generate a hash of a string. You will be prompted if a string isn't provided.
    /// It isn't recommended to put sensitive data as arguments.
    #[command(arg_required_else_help = false)]
    Hash {
        /// The string to hash.
        value: Option<String>,
    },

    /// Output the WIT (WASM Interface Type) definition, for building Slipway components.
    #[command()]
    Wit,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::enum_variant_names)]
enum ServeCommands {
    /// Create basic configuration files and directory structure.
    Init,

    /// Download all required components to the local components folder.
    Consolidate,

    /// Try to ahead-of-time compile any WASM components.
    AotCompile {
        /// Optional target for cross-compilation. Defaults to the host target.
        /// This is a target triple, such as `x86_64-unknown-linux-gnu`.
        #[arg(short, long)]
        target: Option<String>,
    },

    /// Add a device to use when serving HTTP requests.
    #[command(arg_required_else_help = true)]
    AddDevice {
        /// The name of the device.
        /// (lowercase alphanumeric plus underscores).
        #[arg(short, long)]
        name: DeviceName,

        /// The optional playlist to use for the device
        /// (lowercase alphanumeric plus underscores).
        #[arg(short, long)]
        playlist: Option<PlaylistName>,
    },

    /// Add or update a device with TRMNL API details.
    #[command(arg_required_else_help = true)]
    AddTrmnlDevice {
        /// The name of the device.
        /// (lowercase alphanumeric plus underscores).
        #[arg(short, long)]
        name: DeviceName,

        /// The ID the device uses to register itself (typically a MAC address).
        #[arg(long)]
        id: String,

        /// The hashed version of the API key the device uses to authenticate itself.
        #[arg(short('k'), long)]
        hashed_api_key: String,

        /// The optional playlist to use for the device
        /// (lowercase alphanumeric plus underscores).
        #[arg(short, long)]
        playlist: Option<PlaylistName>,
    },

    /// Add a playlist to use when serving HTTP requests.
    #[command(arg_required_else_help = true)]
    AddPlaylist {
        /// A name for the playlist (lowercase alphanumeric plus underscores).
        #[arg(short, long)]
        name: PlaylistName,

        /// The optional name of the rig to populate the playlist with
        /// (lowercase alphanumeric plus underscores).
        #[arg(short, long)]
        rig: Option<RigName>,
    },

    /// Add a rig to use when serving HTTP requests.
    #[command(arg_required_else_help = true)]
    AddRig {
        /// A name for the rig (lowercase alphanumeric plus underscores).
        #[arg(short, long)]
        name: RigName,
    },

    /// Add an API key for accessing endpoints. The key itself will be generated and stored hashed.
    #[command(arg_required_else_help = true)]
    AddApiKey {
        /// A name for the API key, to help you identify it (lowercase alphanumeric plus underscores).
        #[arg(short, long)]
        name: ApiKeyName,
    },
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
    ///   file:../slipway_{name}/components/{publisher}.{name}.{version}.tar
    #[arg(short, long, verbatim_doc_comment)]
    registry_url: Vec<String>,

    #[command(flatten)]
    permissions: CommonPermissionsArgs,
}

enum RuntimeType {
    TokioSingleThread,
    Actix,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let runtime_type = if matches!(
        args.command,
        Commands::Serve {
            subcommand: None,
            ..
        }
    ) {
        RuntimeType::Actix
    } else {
        RuntimeType::TokioSingleThread
    };

    match runtime_type {
        RuntimeType::TokioSingleThread => {
            let mtr = tokio::runtime::Builder::new_multi_thread()
                .enable_io()
                .enable_time()
                .build()?;
            mtr.block_on(async { main_single_threaded(args).await })?;
        }
        RuntimeType::Actix => {
            actix_web::rt::System::new().block_on(async { main_actix_web(args).await })?;
        }
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
        Commands::InitComponent { publisher, name } => {
            let component = slipway_engine::Component {
                publisher,
                name,
                version: Version::new(1, 0, 0),
                description: None,
                input: serde_json::Value::Object(Default::default()),
                output: serde_json::Value::Object(Default::default()),
                constants: None,
                rigging: None,
                callouts: None,
            };

            serde_json::to_writer_pretty(
                std::fs::File::create(SLIPWAY_COMPONENT_FILE_NAME)?,
                &component,
            )?;
        }
        Commands::InitRig { name } => {
            let rig = slipway_engine::Rig {
                description: None,
                constants: None,
                rigging: slipway_engine::Rigging {
                    components: Default::default(),
                },
            };

            serde_json::to_writer_pretty(std::fs::File::create(name.to_string() + ".json")?, &rig)?;
        }
        Commands::Run {
            rig,
            common,
            output,
            output_debug_rig,
            fonts,
        } => {
            let log_level = common.log_level;
            let registry_url = common.registry_url;
            configure_tracing(log_level);
            let permissions = common.permissions.into_permissions()?;
            run_rig::run_rig(
                Box::new(std::io::stdout()),
                rig,
                (&permissions).into(),
                registry_url,
                output,
                output_debug_rig,
                fonts,
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
        Commands::Hash { value } => {
            configure_tracing(Default::default());
            let value = value.unwrap_or_else(|| {
                rpassword::prompt_password("Enter the string to hash: ")
                    .expect("Should be able to read a secret value")
            });
            if value.len() > 3 {
                info!(
                    "Hashing value starting \"{}\" and ending \"{}\"",
                    &value[..3],
                    &value[value.len() - 3..]
                );
            } else {
                info!("Hashing value of length {}", value.len());
            }
            println!("{}", hash_string(&value));
        }
        Commands::Wit => {
            println!("{}", WASM_INTERFACE_TYPE_STR);
        }
        Commands::Serve {
            path,
            aot: _,
            subcommand,
        } => match subcommand {
            Some(ServeCommands::Init) => {
                configure_tracing(Default::default());
                serve::commands::init(path).await?;
            }
            Some(ServeCommands::Consolidate) => {
                configure_tracing(Some("debug".to_string()));
                serve::commands::consolidate(path).await?;
            }
            Some(ServeCommands::AotCompile { target }) => {
                configure_tracing(Some("debug".to_string()));
                let aot_path = path.join(AOT_ARTIFACT_FOLDER_NAME);
                let cache = serve::commands::consolidate(path.clone()).await?;
                serve::commands::aot_compile(aot_path, target.as_deref(), cache).await?;
            }
            Some(ServeCommands::AddDevice { name, playlist }) => {
                configure_tracing(Default::default());
                serve::commands::add_device(path, name, playlist).await?;
            }
            Some(ServeCommands::AddTrmnlDevice {
                id,
                hashed_api_key,
                name,
                playlist,
            }) => {
                configure_tracing(Default::default());
                serve::commands::add_trmnl_device(path, id, hashed_api_key, name, playlist).await?;
            }
            Some(ServeCommands::AddPlaylist { name, rig }) => {
                configure_tracing(Default::default());
                serve::commands::add_playlist(path, name, rig).await?;
            }
            Some(ServeCommands::AddRig { name }) => {
                configure_tracing(Default::default());
                serve::commands::add_rig(path, name).await?;
            }
            Some(ServeCommands::AddApiKey { name }) => {
                configure_tracing(Default::default());
                serve::commands::add_api_key(path, name, None).await?;
            }
            None => {
                panic!(
                    "Serve command with no subcommand is not supported in single-threaded mode."
                );
            }
        },
    }

    Ok(())
}

async fn main_actix_web(args: Cli) -> anyhow::Result<()> {
    // Note we're not setting a ctrl+c handler here because
    // the server will handle it.
    match args.command {
        Commands::Serve {
            path,
            aot,
            subcommand: None,
        } => {
            let aot_path = if aot {
                Some(path.join(AOT_ARTIFACT_FOLDER_NAME))
            } else {
                None
            };
            serve::serve(path, aot_path).await?;
        }
        _ => {
            panic!("Command is not supported in actix-web mode.");
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
