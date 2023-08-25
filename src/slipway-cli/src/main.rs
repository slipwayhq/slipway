mod cli;

use clap::Parser;
use cli::{Cli, Commands};
use slipway_lib::rigging::{parse::parse_component, validate::validate_component};

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::ValidateRigging { input } => {
            validate_rigging_command(input)?;
        }
    }

    Ok(())
}

fn validate_rigging_command(input: std::path::PathBuf) -> anyhow::Result<()> {
    println!("Validating {}", input.display());
    let file_contents = std::fs::read_to_string(input)?;
    let component = parse_component(file_contents.as_str())?;
    let failures = validate_component(None, &component).failures;
    if !failures.is_empty() {
        println!("Rigging was invalid");
        for failure in failures {
            println!("{}", failure);
        }
        std::process::exit(1);
    }
    println!("Rigging was valid");
    Ok(())
}
