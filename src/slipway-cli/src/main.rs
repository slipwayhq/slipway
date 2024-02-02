mod cli;

use std::collections::{HashMap, HashSet};

use clap::Parser;
use cli::{Cli, Commands};
use slipway_lib::{create_app_from_json_string, ComponentHandle};
// use slipway_lib::rigging_v1::{
//     parse::{parse_component, types::UnresolvedComponentReference},
//     validate::validate_component,
// };

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
    let state = create_app_from_json_string(&file_contents)?;
    let valid_instructions = state.get_valid_instructions();

    let graph = state.get_dependencies();

    for (component, dependencies) in graph {
        println!("{} depends on:", component);
        print_dependencies(component, dependencies, graph, 1);
    }
    Ok(())
}

fn print_dependencies(
    component: &ComponentHandle,
    dependencies: &HashSet<ComponentHandle>,
    graph: &HashMap<ComponentHandle, HashSet<ComponentHandle>>,
    level: usize,
) {
    for dependency in dependencies {
        println!("{}- {}", " ".repeat(level * 4), dependency);
        if let Some(sub_dependencies) = graph.get(dependency) {
            print_dependencies(dependency, sub_dependencies, graph, level + 1);
        }
    }
}
