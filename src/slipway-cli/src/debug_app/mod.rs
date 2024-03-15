use clap::{Arg, ArgMatches, Command};
use std::io::{self, ErrorKind, Write};
use termion::{color, style};

use slipway_lib::{
    parse_app, AppExecutionState, AppSession, ComponentHandle, Immutable, Instruction,
};

use crate::to_view_model::{to_view_model, AppExecutionStateViewModel};
use crate::write_app_state;

use self::errors::SlipwayDebugError;

mod errors;

pub(crate) fn debug_app(input: std::path::PathBuf) -> anyhow::Result<()> {
    println!("Debugging {}", input.display());
    println!();
    let file_contents = std::fs::read_to_string(input)?;
    let app = parse_app(&file_contents)?;
    let session = AppSession::from(app);
    let state = session.initialize()?;

    let stdout = std::io::stdout();
    let mut stdout_handle = stdout.lock();

    let mut view_model = to_view_model(state);

    write_app_state::write_app_state(&mut stdout_handle, &view_model)?;
    println!();

    // Set the Ctrl+C handler
    ctrlc::set_handler(move || {
        std::process::exit(1);
    })
    .expect("Error setting Ctrl-C handler");

    let command = Command::new("Slipway Interactive Debugger")
        .subcommand(
            Command::new("run")
                .about("Runs a component")
                .arg(Arg::new("handle").required(true)),
        )
        .subcommand(
            Command::new("input")
                .about("Edits the input of a component")
                .arg(Arg::new("handle").required(true)),
        )
        .subcommand(
            Command::new("output")
                .about("Edits the output of a component")
                .arg(Arg::new("handle").required(true)),
        )
        .subcommand(
            Command::new("clear")
                .about("Clears either the input or output override of a component")
                .subcommand(
                    Command::new("input")
                        .about("Clears the input override of a component")
                        .arg(Arg::new("handle").required(true)),
                )
                .subcommand(
                    Command::new("output")
                        .about("Clears the output override of a component")
                        .arg(Arg::new("handle").required(true)),
                )
                .subcommand_required(true)
                .infer_subcommands(true),
        )
        .subcommand(Command::new("exit").about("Exits the debugger"))
        .subcommand_required(true)
        .infer_subcommands(true);

    let help_color = color::Fg(color::Yellow);
    println!(
        "{}Type {}help{}{} for commands.{}",
        help_color,
        style::Underline,
        style::Reset,
        help_color,
        color::Fg(color::Reset)
    );

    loop {
        print!("{}>> {}", color::Fg(color::Green), color::Fg(color::Reset));
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let mut args = input.split_whitespace().collect::<Vec<&str>>();
            if args.is_empty() {
                continue;
            }

            args.insert(0, "slipway");

            match command.clone().try_get_matches_from(args) {
                Ok(matches) => match handle_command(matches, &view_model) {
                    Ok(HandleCommandResult::Continue(Some(new_state))) => {
                        view_model = to_view_model(new_state);
                    }
                    Ok(HandleCommandResult::Continue(None)) => {}
                    Ok(HandleCommandResult::Exit) => break,
                    Err(e) => println!("{}{}{}", color::Fg(color::Red), e, color::Fg(color::Reset)),
                },
                Err(e) => e.print().expect("Parsing errors should be printed"), // Display parsing errors
            }
        } else {
            println!("Error reading input");
        }
    }

    println!("Exiting application...");

    Ok(())
}

enum HandleCommandResult<'app> {
    Continue(Option<Immutable<AppExecutionState<'app>>>),
    Exit,
}

fn handle_command<'app>(
    matches: ArgMatches,
    view_model: &AppExecutionStateViewModel<'app>,
) -> Result<HandleCommandResult<'app>, SlipwayDebugError> {
    if let Some(matches) = matches.subcommand_matches("input") {
        let handle = get_handle(matches, view_model)?;
        let new_state = handle_input_command(handle, &view_model.state)?;
        return Ok(HandleCommandResult::Continue(Some(new_state)));
    } else if let Some(matches) = matches.subcommand_matches("output") {
        let handle = get_handle(matches, view_model)?;
        println!("Edit the output of {}", handle);
    } else if let Some(matches) = matches.subcommand_matches("run") {
        let handle = get_handle(matches, view_model)?;
        println!("Run {}", handle);
    } else if let Some(matches) = matches.subcommand_matches("clear") {
        if let Some(matches) = matches.subcommand_matches("input") {
            let handle = get_handle(matches, view_model)?;
            println!("Clear input override for {}", handle);
        } else if let Some(matches) = matches.subcommand_matches("output") {
            let handle = get_handle(matches, view_model)?;
            println!("Clear output override for {}", handle);
        }
    } else if matches.subcommand_matches("exit").is_some() {
        return Ok(HandleCommandResult::Exit);
    }

    Ok(HandleCommandResult::Continue(None))
}

fn handle_input_command<'app>(
    handle: ComponentHandle,
    state: &AppExecutionState<'app>,
) -> Result<Immutable<AppExecutionState<'app>>, SlipwayDebugError> {
    let component = state
        .component_states
        .get(&handle)
        .expect("Component should exist");

    let template = component.input().ok_or_else(|| {
        SlipwayDebugError::UserError(format!("Component {} has no input", handle))
    })?;

    let new_input = edit_json(template)?;

    let new_state = state.step(Instruction::SetInputOverride {
        handle,
        value: new_input,
    })?;

    Ok(new_state)
}

fn edit_json(template: &serde_json::Value) -> Result<serde_json::Value, SlipwayDebugError> {
    let template_string =
        serde_json::to_string_pretty(&template).expect("Component input should be serializable");
    let maybe_edited = edit::edit(template_string);
    match maybe_edited {
        Ok(edited) => {
            let result = serde_json::from_str(&edited)?;
            Ok(result)
        }
        Err(e) => match e.kind() {
            ErrorKind::InvalidData => Err(SlipwayDebugError::UserError(
                "Could not decode input as UTF-8".into(),
            )),
            ErrorKind::NotFound => {
                Err(SlipwayDebugError::UserError("Text editor not found".into()))
            }
            other_error => Err(SlipwayDebugError::UserError(format!(
                "Failed to open the file: {:?}",
                other_error
            ))),
        },
    }
}

fn get_handle(
    matches: &clap::ArgMatches,
    view_model: &AppExecutionStateViewModel<'_>,
) -> Result<ComponentHandle, SlipwayDebugError> {
    let handle_str = matches
        .get_one::<String>("handle")
        .expect("Handle is required");

    // Find the first component whose handle matches handle_str or whose shortcut matches handle_str.
    view_model
        .groups
        .iter()
        .flat_map(|g| g.components.iter())
        .find(|c| &c.handle.0 == handle_str || &c.shortcut == handle_str)
        .map(|c| c.handle.clone())
        .ok_or_else(|| {
            SlipwayDebugError::UserError(format!(
                "No component found with handle or shortcut {}",
                handle_str
            ))
        })
}
