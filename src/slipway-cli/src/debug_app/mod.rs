use clap::{Arg, ArgMatches, Command};
use std::io::{self, ErrorKind, Write};
use termion::{color, style};

use slipway_lib::{
    parse_app, AppExecutionState, AppSession, ComponentHandle, Immutable, Instruction,
};

use crate::to_view_model::{to_shortcuts, to_view_model};
use crate::write_app_state;

use self::errors::SlipwayDebugError;

mod errors;

pub(crate) fn debug_app(input: std::path::PathBuf) -> anyhow::Result<()> {
    set_ctrl_c_handler();

    println!("Debugging {}", input.display());
    println!();
    let file_contents = std::fs::read_to_string(input)?;
    let app = parse_app(&file_contents)?;
    let session = AppSession::from(app);
    let mut state = session.initialize()?;

    print_state(&state)?;

    let command = create_command_structure();

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
                Ok(matches) => match handle_command(matches, &state) {
                    Ok(HandleCommandResult::Continue(Some(s))) => {
                        state = s;
                        print_state(&state)?;
                    }
                    Ok(HandleCommandResult::Continue(None)) => {}
                    Ok(HandleCommandResult::Exit) => break,
                    Err(e) => {
                        println!("{}{}{}", color::Fg(color::Red), e, color::Fg(color::Reset));
                        print_state(&state)?;
                    }
                },
                Err(e) => e.print().expect("Parsing errors should be printed"),
            }
        } else {
            println!("Error reading input");
        }
    }

    println!("Exiting application...");

    Ok(())
}

fn set_ctrl_c_handler() {
    ctrlc::set_handler(move || {
        std::process::exit(1);
    })
    .expect("Error setting Ctrl-C handler");
}

const RUN_COMMAND: &str = "run";
const INPUT_COMMAND: &str = "input";
const OUTPUT_COMMAND: &str = "output";
const CLEAR_COMMAND: &str = "clear";
const EXIT_COMMAND: &str = "exit";

fn create_command_structure() -> Command {
    Command::new("Slipway Interactive Debugger")
        .subcommand(
            Command::new(RUN_COMMAND)
                .about("Runs a component")
                .arg(Arg::new("handle").required(true)),
        )
        .subcommand(
            Command::new(INPUT_COMMAND)
                .about("Edits the input of a component")
                .arg(Arg::new("handle").required(true)),
        )
        .subcommand(
            Command::new(OUTPUT_COMMAND)
                .about("Edits the output of a component")
                .arg(Arg::new("handle").required(true)),
        )
        .subcommand(
            Command::new(CLEAR_COMMAND)
                .about("Clears either the input or output override of a component")
                .subcommand(
                    Command::new(INPUT_COMMAND)
                        .about("Clears the input override of a component")
                        .arg(Arg::new("handle").required(true)),
                )
                .subcommand(
                    Command::new(OUTPUT_COMMAND)
                        .about("Clears the output override of a component")
                        .arg(Arg::new("handle").required(true)),
                )
                .subcommand_required(true)
                .infer_subcommands(true),
        )
        .subcommand(Command::new(EXIT_COMMAND).about("Exits the debugger"))
        .subcommand_required(true)
        .infer_subcommands(true)
}

enum HandleCommandResult<'app> {
    Continue(Option<Immutable<AppExecutionState<'app>>>),
    Exit,
}

fn handle_command<'app, 'state>(
    matches: ArgMatches,
    state: &'state AppExecutionState<'app>,
) -> anyhow::Result<HandleCommandResult<'app>> {
    let result: HandleCommandResult<'app> =
        if let Some(matches) = matches.subcommand_matches(INPUT_COMMAND) {
            let handle = get_handle(matches, state)?;
            let new_state = handle_input_command(handle, state)?;
            HandleCommandResult::Continue(Some(new_state))
        } else if let Some(matches) = matches.subcommand_matches(OUTPUT_COMMAND) {
            let handle = get_handle(matches, state)?;
            println!("Edit the output of {}", handle);
            HandleCommandResult::Continue(None)
        } else if let Some(matches) = matches.subcommand_matches(RUN_COMMAND) {
            let handle = get_handle(matches, state)?;
            println!("Run {}", handle);
            HandleCommandResult::Continue(None)
        } else if let Some(matches) = matches.subcommand_matches(CLEAR_COMMAND) {
            if let Some(matches) = matches.subcommand_matches(INPUT_COMMAND) {
                let handle = get_handle(matches, state)?;
                println!("Clear input override for {}", handle);
                HandleCommandResult::Continue(None)
            } else if let Some(matches) = matches.subcommand_matches(OUTPUT_COMMAND) {
                let handle = get_handle(matches, state)?;
                println!("Clear output override for {}", handle);
                HandleCommandResult::Continue(None)
            } else {
                HandleCommandResult::Continue(None)
            }
        } else if matches.subcommand_matches(EXIT_COMMAND).is_some() {
            HandleCommandResult::Exit
        } else {
            HandleCommandResult::Continue(None)
        };

    Ok(result)
}

fn print_state(state: &AppExecutionState<'_>) -> Result<(), anyhow::Error> {
    let view_model = to_view_model(state);
    write_app_state::write_app_state(&mut io::stdout(), &view_model)?;
    println!();
    Ok(())
}

fn handle_input_command<'app>(
    handle: &'app ComponentHandle,
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
        handle: handle.clone(),
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

fn get_handle<'app>(
    matches: &clap::ArgMatches,
    state: &AppExecutionState<'app>,
) -> Result<&'app ComponentHandle, SlipwayDebugError> {
    let handle_str = matches
        .get_one::<String>("handle")
        .expect("Handle is required");

    let shortcuts = to_shortcuts(state);

    if let Some(&handle) = shortcuts.get(handle_str) {
        return Ok(handle);
    }

    Err(SlipwayDebugError::UserError(format!(
        "No component found for handle or shortcut {}",
        handle_str
    )))
}
