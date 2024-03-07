#![allow(dead_code)]

mod cli;
mod utils;
mod write_app_state;

use clap::Parser;
use cli::{Cli, Commands};
use slipway_lib::{parse_app, AppSession};

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::DebugApp { input } => {
            debug_app_command(input)?;
        }
    }

    Ok(())
}

fn debug_app_command(input: std::path::PathBuf) -> anyhow::Result<()> {
    println!("Debugging {}", input.display());
    let file_contents = std::fs::read_to_string(input)?;
    let app = parse_app(&file_contents)?;
    let session = AppSession::from(app);
    let state = session.initialize()?;
    // let components = state.component_states();

    // Create stdout writer.
    let stdout = std::io::stdout();
    let mut stdout_handle = stdout.lock();

    write_app_state::write_app_state(&mut stdout_handle, &state)?;
    // let graph = components
    //     .iter()
    //     .map(|c| (c.handle, c.dependencies.clone()))
    //     .collect();

    // for component in components.iter() {
    //     println!("{} depends on:", component.handle);
    //     print_dependencies(&component.dependencies, &graph, 1);
    // }
    Ok(())
}
