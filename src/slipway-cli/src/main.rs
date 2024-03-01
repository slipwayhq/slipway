mod cli;
mod print_app_state;

use std::collections::{HashMap, HashSet};

use clap::Parser;
use cli::{Cli, Commands};
use slipway_lib::{parse_app, AppSession, ComponentHandle};

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
    let components = state.component_states();

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

fn print_dependencies(
    dependencies: &HashSet<&ComponentHandle>,
    graph: &HashMap<&ComponentHandle, HashSet<&ComponentHandle>>,
    level: usize,
) {
    for dependency in dependencies {
        println!("{}- {}", " ".repeat(level * 4), dependency);
        if let Some(sub_dependencies) = graph.get(dependency) {
            print_dependencies(sub_dependencies, graph, level + 1);
        }
    }
}
