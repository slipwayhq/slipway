use clap::{Arg, Command};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use termion::{color, style};

use slipway_lib::{parse_app, AppSession};

use crate::write_app_state;

pub(crate) fn debug_app(input: std::path::PathBuf) -> anyhow::Result<()> {
    println!("Debugging {}", input.display());
    println!();
    let file_contents = std::fs::read_to_string(input)?;
    let app = parse_app(&file_contents)?;
    let session = AppSession::from(app);
    let state = session.initialize()?;

    let stdout = std::io::stdout();
    let mut stdout_handle = stdout.lock();

    write_app_state::write_app_state(&mut stdout_handle, &state)?;
    println!();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Set the Ctrl+C handler
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
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

    while running.load(Ordering::SeqCst) {
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
                Ok(matches) => {
                    if let Some(matches) = matches.subcommand_matches("input") {
                        if let Some(handle) = matches.get_one::<String>("handle") {
                            println!("Edit the input of {}", handle);
                        }
                    } else if let Some(matches) = matches.subcommand_matches("output") {
                        if let Some(handle) = matches.get_one::<String>("handle") {
                            println!("Edit the output of {}", handle);
                        }
                    } else if let Some(matches) = matches.subcommand_matches("run") {
                        if let Some(handle) = matches.get_one::<String>("handle") {
                            println!("Run {}", handle);
                        }
                    } else if let Some(matches) = matches.subcommand_matches("clear") {
                        if let Some(matches) = matches.subcommand_matches("input") {
                            if let Some(handle) = matches.get_one::<String>("handle") {
                                println!("Clear input override for {}", handle);
                            }
                        } else if let Some(matches) = matches.subcommand_matches("output") {
                            if let Some(handle) = matches.get_one::<String>("handle") {
                                println!("Clear output override for {}", handle);
                            }
                        }
                    } else if matches.subcommand_matches("exit").is_some() {
                        running.store(false, Ordering::SeqCst);
                    }
                }
                Err(e) => e.print().unwrap(), // Display parsing errors
            }
        } else {
            println!("Error reading input");
        }
    }

    println!("Exiting application...");

    Ok(())
}
