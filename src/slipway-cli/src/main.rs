mod cli;

use std::collections::HashMap;

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

    let components = state.component_states();

    let graph = components
        .iter()
        .map(|c| (c.handle.clone(), c.dependencies.clone()))
        .collect();

    for component in components.iter() {
        println!("{} depends on:", component.handle);
        print_dependencies(&component.dependencies, &graph, 1);
    }
    Ok(())
}

fn print_dependencies(
    dependencies: &[ComponentHandle],
    graph: &HashMap<ComponentHandle, Vec<ComponentHandle>>,
    level: usize,
) {
    for dependency in dependencies {
        println!("{}- {}", " ".repeat(level * 4), dependency);
        if let Some(sub_dependencies) = graph.get(dependency) {
            print_dependencies(sub_dependencies, graph, level + 1);
        }
    }
}
