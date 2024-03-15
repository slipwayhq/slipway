use std::io::Write;

use termion::{color, style};

use crate::{
    to_view_model::{AppExecutionStateViewModel, ComponentGroupViewModel, ComponentViewModel},
    utils::{format_bytes, skip_first_n_chars},
};

const HASH_RENDER_CHAR_COUNT: usize = 8;
const COLUMN_PADDING: &str = "  ";
const COLUMN_CHAR: char = '┆';

// • 0
// │ • 1
// │ │ • 2
// ├─│─│─• 5
// ├─│─│─│─• 6
// ╰─│─│─┼─│─• 4
//   │ │ ├─│─│─• 8
//   │ │ │ │ ╰─│─• 7
//   │ │ ╰─│───┴─┴─• 9
//   ╰─┴───┴───────┴─• 10
// • 3

pub(crate) fn write_app_state<F: Write>(
    f: &mut F,
    view_model: &AppExecutionStateViewModel<'_>,
) -> anyhow::Result<()> {
    let max_component_state_row_length = get_max_component_state_row_length(view_model);
    let max_input_size_string_length = get_max_input_size_string_length(view_model);
    let max_output_size_string_length = get_max_output_size_string_length(view_model);

    for group in view_model.groups.iter() {
        for component in group.components.iter() {
            // write!(f, "{}", COLUMN_CHAR)?;
            // write!(f, "{}", COLUMN_PADDING)?;

            write_component_state(f, view_model, component, group)?;

            let padding_required = max_component_state_row_length // The longest state length
                - component.handle.0.len() // Subtract the handle length
                - 2 * component.row_index; // Subtract twice the row index (the tree structure chars)
            write!(f, "{}", " ".repeat(padding_required))?;

            write!(f, "{}", COLUMN_PADDING)?;
            write!(f, "{}", COLUMN_CHAR)?;
            write!(f, "{}", COLUMN_PADDING)?;

            write_metadata(f, view_model, component, MetadataType::Hashes)?;

            write!(f, "{}", COLUMN_PADDING)?;
            write!(f, "{}", COLUMN_CHAR)?;
            write!(f, "{}", COLUMN_PADDING)?;

            write_metadata(
                f,
                view_model,
                component,
                MetadataType::Sizes {
                    max_input_size_string_length,
                    max_output_size_string_length,
                },
            )?;

            // write!(f, "{}", COLUMN_PADDING)?;
            // write!(f, "{}", COLUMN_CHAR)?;

            writeln!(f)?;
        }
    }
    Ok(())
}

/// Returns the length of the longest component state row, including the tree structure,
/// excluding metadata like hashes and sizes.
fn get_max_component_state_row_length(view_model: &AppExecutionStateViewModel<'_>) -> usize {
    view_model
        .groups
        .iter()
        .flat_map(|g| {
            g.components
                .iter()
                .map(|c| c.row_index * 2 + c.handle.0.len())
        })
        .max()
        .unwrap_or(0)
}

/// Returns the length of the longest input size string.
fn get_max_input_size_string_length(view_model: &AppExecutionStateViewModel<'_>) -> usize {
    view_model
        .groups
        .iter()
        .flat_map(|g| {
            g.components.iter().map(|c| {
                c.state(view_model)
                    .execution_input
                    .as_ref()
                    .map(|i| format_bytes(i.metadata.serialized.len()).len())
                    .unwrap_or_default()
            })
        })
        .max()
        .unwrap_or(0)
}

/// Returns the length of the longest output size string.
fn get_max_output_size_string_length(view_model: &AppExecutionStateViewModel<'_>) -> usize {
    view_model
        .groups
        .iter()
        .flat_map(|g| {
            g.components.iter().map(|c| {
                c.state(view_model)
                    .output_override
                    .as_ref()
                    .map(|i| format_bytes(i.metadata.serialized.len()).len())
                    .unwrap_or(
                        c.state(view_model)
                            .execution_output
                            .as_ref()
                            .map(|i| format_bytes(i.metadata.serialized.len()).len())
                            .unwrap_or_default(),
                    )
            })
        })
        .max()
        .unwrap_or(0)
}

fn write_component_state<F: Write>(
    f: &mut F,
    view_model: &AppExecutionStateViewModel<'_>,
    component: &ComponentViewModel<'_>,
    group: &ComponentGroupViewModel<'_>,
) -> anyhow::Result<()> {
    const NO_INPUT_NO_OUTPUT: char = '□';
    const INPUT_NO_OUTPUT: char = '◩';
    const NO_INPUT_OUTPUT: char = '◪';
    const INPUT_OUTPUT: char = '■';

    for i in 0..component.row_index {
        let i_component = &group.components[i];
        let i_component_last_output = i_component.output_row_indexes.last();
        let component_first_input = component.input_columns_indexes.first();

        let is_input = component.input_columns_indexes.contains(&i);
        if is_input {
            // └ ├ ┼ ┴
            if match i_component_last_output {
                Some(&last_output) => last_output > component.row_index,
                None => false,
            } {
                //├ ┼
                if match component_first_input {
                    Some(&first_input) => first_input < i,
                    None => false,
                } {
                    write!(f, "┼─")?;
                } else {
                    write!(f, "├─")?;
                }
            } else if match component_first_input {
                Some(&first_input) => first_input < i,
                None => false,
            } {
                write!(f, "┴─")?;
            } else {
                write!(f, "└─")?;
            }
        } else {
            // │ ─ " "
            if match i_component_last_output {
                Some(&last_output) => last_output > component.row_index,
                None => false,
            } {
                write!(f, "│─")?;
            } else if match component_first_input {
                Some(&first_input) => first_input < i,
                None => false,
            } {
                write!(f, "──")?;
            } else {
                write!(f, "  ")?;
            }
        }
    }
    let component_color = match component.state(view_model).output() {
        Some(_) => match component.state(view_model).execution_input {
            Some(_) => ComponentColors::HasInputAndOutput,
            None => ComponentColors::HasOutput,
        },
        None => match component.state(view_model).execution_input {
            Some(_) => ComponentColors::HasInput,
            None => ComponentColors::Default,
        },
    };
    component_color.write_foreground(f)?;
    match component_color {
        ComponentColors::HasInput => write!(f, "{}", INPUT_NO_OUTPUT)?,
        ComponentColors::HasOutput => write!(f, "{}", NO_INPUT_OUTPUT)?,
        ComponentColors::HasInputAndOutput => write!(f, "{}", INPUT_OUTPUT)?,
        _ => write!(f, "{}", NO_INPUT_NO_OUTPUT)?,
    }
    write!(f, "{}", color::Fg(color::Reset))?;
    write!(f, " ")?;
    write!(f, "{}", style::Underline)?;
    write!(f, "{}", component.shortcut)?;
    write!(f, "{}", style::Reset)?;
    write!(
        f,
        "{}",
        skip_first_n_chars(&component.handle.0, component.shortcut.len())
    )?;
    Ok(())
}

enum MetadataType {
    Hashes,
    Sizes {
        max_input_size_string_length: usize,
        max_output_size_string_length: usize,
    },
}

fn write_metadata<F: Write>(
    f: &mut F,
    view_model: &AppExecutionStateViewModel<'_>,
    component: &ComponentViewModel<'_>,
    metadata_type: MetadataType,
) -> anyhow::Result<()> {
    const OUTPUT_FROM_INPUT: char = '➜';
    const OUTPUT_NOT_FROM_INPUT: char = '!';
    const NO_OUTPUT: char = ' ';

    if let Some(input) = &component.state(view_model).execution_input {
        let should_underline = component.state(view_model).input_override.is_some();
        write!(f, "{}", color::Fg(color::Blue))?;

        match metadata_type {
            MetadataType::Hashes => {
                let input_hash_string =
                    format!("{}", input.metadata.hash)[..HASH_RENDER_CHAR_COUNT].to_string();

                if should_underline {
                    write!(
                        f,
                        "{}{}{}",
                        style::Underline,
                        input_hash_string,
                        style::Reset
                    )?;
                } else {
                    write!(f, "{}", input_hash_string)?
                }
            }
            MetadataType::Sizes {
                max_input_size_string_length,
                max_output_size_string_length: _,
            } => {
                let input_size_string = format_bytes(input.metadata.serialized.len());
                let padding_required = max_input_size_string_length - input_size_string.len();
                write!(f, "{:padding_required$}", "")?;
                if should_underline {
                    write!(
                        f,
                        "{}{}{}",
                        style::Underline,
                        input_size_string,
                        style::Reset
                    )?;
                } else {
                    write!(f, "{}", input_size_string)?;
                }
            }
        }

        write!(f, "{}", color::Fg(color::Reset))?;
    } else {
        match metadata_type {
            MetadataType::Hashes => write!(f, "{:HASH_RENDER_CHAR_COUNT$}", "")?,
            MetadataType::Sizes {
                max_input_size_string_length,
                max_output_size_string_length: _,
            } => {
                write!(f, "{:max_input_size_string_length$}", "")?;
            }
        }
    }

    if component.state(view_model).output().is_some() {
        let (color, hash, size): (ComponentColors, &slipway_lib::Hash, usize) = {
            if let Some(output_override) = &component.state(view_model).output_override {
                (
                    ComponentColors::HashesIgnored,
                    &output_override.metadata.hash,
                    output_override.metadata.serialized.len(),
                )
            } else {
                let execution_output = &component
                    .state(view_model)
                    .execution_output
                    .as_ref()
                    .expect("Either execution_output or output_override should exist");
                let output_hash = &execution_output.metadata.hash;
                let output_size = execution_output.metadata.serialized.len();
                if let Some(execution_input) = &component.state(view_model).execution_input {
                    if execution_input.metadata.hash == execution_output.input_hash_used {
                        (ComponentColors::HashesMatch, output_hash, output_size)
                    } else {
                        (ComponentColors::HashesDiffer, output_hash, output_size)
                    }
                } else {
                    (ComponentColors::HashesIgnored, output_hash, output_size)
                }
            }
        };

        match color {
            ComponentColors::HashesMatch => write!(f, " {} ", OUTPUT_FROM_INPUT)?,
            _ => write!(f, " {} ", OUTPUT_NOT_FROM_INPUT)?,
        }

        let should_underline = component.state(view_model).output_override.is_some();

        color.write_foreground(f)?;

        match metadata_type {
            MetadataType::Hashes => {
                let output_hash_string = &format!("{}", hash)[..HASH_RENDER_CHAR_COUNT];
                if should_underline {
                    write!(
                        f,
                        "{}{}{}",
                        style::Underline,
                        output_hash_string,
                        style::Reset
                    )?;
                } else {
                    write!(f, "{}", output_hash_string)?
                }
            }
            MetadataType::Sizes {
                max_input_size_string_length: _,
                max_output_size_string_length,
            } => {
                let size_string = format_bytes(size);
                let padding_required = max_output_size_string_length - size_string.len();
                write!(f, "{:padding_required$}", "")?;
                if should_underline {
                    write!(f, "{}{}{}", style::Underline, size_string, style::Reset)?;
                } else {
                    write!(f, "{}", size_string)?;
                }
            }
        }

        write!(f, "{}", color::Fg(color::Reset))?;
    } else {
        write!(f, " {} ", NO_OUTPUT)?;

        match metadata_type {
            MetadataType::Hashes => write!(f, "{:HASH_RENDER_CHAR_COUNT$}", "")?,
            MetadataType::Sizes {
                max_input_size_string_length: _,
                max_output_size_string_length,
            } => {
                write!(f, "{:max_output_size_string_length$}", "")?;
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
enum ComponentColors {
    Default,
    HasOutput,
    HasInput,
    HasInputAndOutput,
    HashesMatch,
    HashesDiffer,
    HashesIgnored,
}

impl ComponentColors {
    fn write_foreground<F: Write>(&self, f: &mut F) -> std::io::Result<()> {
        match self {
            ComponentColors::Default => write!(f, "{}", color::Fg(color::White)),
            ComponentColors::HasOutput => write!(f, "{}", color::Fg(color::Green)),
            ComponentColors::HasInput => write!(f, "{}", color::Fg(color::Yellow)),
            ComponentColors::HasInputAndOutput => write!(f, "{}", color::Fg(color::Green)),
            ComponentColors::HashesMatch => write!(f, "{}", color::Fg(color::Green)),
            ComponentColors::HashesDiffer => write!(f, "{}", color::Fg(color::Red)),
            ComponentColors::HashesIgnored => write!(f, "{}", color::Fg(color::Blue)),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use slipway_lib::{utils::ch, App, AppSession, ComponentRigging, Instruction, Rigging};

    use crate::to_view_model::to_view_model;

    use super::*;

    #[test]
    fn it_should_indicate_component_states() {
        // ■ ant
        // ├─◩ bird
        // └─│─◩ cat
        //   └─┴─□ duck
        // ◩ asp
        // └─◪ bull
        // ◩ assassin_bug
        let app = App::for_test(Rigging {
            components: [
                ComponentRigging::for_test("ant", None),
                ComponentRigging::for_test("bird", Some(json!({"a": "$$.ant"}))),
                ComponentRigging::for_test("cat", Some(json!({"a": "$$.ant"}))),
                ComponentRigging::for_test("duck", Some(json!({"b": "$$.bird", "c": "$$.cat"}))),
                ComponentRigging::for_test("asp", None),
                ComponentRigging::for_test("bull", Some(json!({"a": "$$.asp"}))),
                ComponentRigging::for_test("assassin_bug", None),
            ]
            .into_iter()
            .collect(),
        });

        let app_session = AppSession::from(app);
        let mut state = app_session.initialize().unwrap();

        state = state
            .step(Instruction::SetOutputOverride {
                handle: ch("ant"),
                value: json!(0),
            })
            .unwrap();

        state = state
            .step(Instruction::SetOutputOverride {
                handle: ch("bull"),
                value: json!(0),
            })
            .unwrap();

        let view_model = to_view_model(state);

        let mut buffer = Vec::new();
        write_app_state(&mut buffer, &view_model).unwrap();
        let buffer_string = String::from_utf8(buffer).unwrap();
        println!("{}", buffer_string);

        let mut lines = buffer_string.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), 7);

        fn state_start_index(_: &str) -> usize {
            0
            // let search_string = format!("{}  ", COLUMN_CHAR);
            // s.find(&search_string).unwrap() + search_string.len()
        }

        fn state_end_index(s: &str) -> usize {
            let search_string = format!("  {}", COLUMN_CHAR);
            s.find(&search_string).unwrap()
        }

        fn get_next_line<'a>(lines: &mut Vec<&'a str>) -> &'a str {
            let line = lines.remove(0);
            &line[state_start_index(line)..state_end_index(line)]
        }

        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "{}■{} {}a{}nt         ",
                color::Fg(color::Green),
                color::Fg(color::Reset),
                style::Underline,
                style::Reset,
            ),
        );

        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "├─{}◩{} {}b{}ird      ",
                color::Fg(color::Yellow),
                color::Fg(color::Reset),
                style::Underline,
                style::Reset,
            ),
        );

        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "└─│─{}◩{} {}c{}at     ",
                color::Fg(color::Yellow),
                color::Fg(color::Reset),
                style::Underline,
                style::Reset,
            ),
        );

        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "  └─┴─{}□{} {}d{}uck  ",
                color::Fg(color::White),
                color::Fg(color::Reset),
                style::Underline,
                style::Reset,
            ),
        );

        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "{}◩{} {}as{}p         ",
                color::Fg(color::Yellow),
                color::Fg(color::Reset),
                style::Underline,
                style::Reset,
            ),
        );

        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "└─{}◪{} {}bu{}ll      ",
                color::Fg(color::Green),
                color::Fg(color::Reset),
                style::Underline,
                style::Reset,
            ),
        );

        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "{}◩{} {}ass{}assin_bug",
                color::Fg(color::Yellow),
                color::Fg(color::Reset),
                style::Underline,
                style::Reset,
            ),
        );
    }

    #[test]
    fn it_should_indicate_hash_states() {
        // ■ ant               ┆  44136fa3 ➜ 6b86b273  ┆   2 bytes ➜ 1 byte   ┆
        // ├─■ bird            ┆  015abd7f ! 5feceb66  ┆   7 bytes ! 1 byte   ┆
        // └─│─■ cat           ┆  015abd7f ! 5feceb66  ┆   7 bytes ! 1 byte   ┆
        //   └─┴─■ duck        ┆  53779b51 ➜ 5feceb66  ┆  13 bytes ➜ 1 byte   ┆
        //       └─◩ elk       ┆  b852cecd             ┆   7 bytes            ┆
        //         └─◪ fish    ┆           ! eadd1967  ┆           ! 4.88 kb  ┆
        //           └─◩ goat  ┆  10c2cd2c             ┆   4.89 kb            ┆
        let app = App::for_test(Rigging {
            components: [
                ComponentRigging::for_test("ant", None),
                ComponentRigging::for_test("bird", Some(json!({"a": "$$.ant"}))),
                ComponentRigging::for_test("cat", Some(json!({"a": "$$.ant"}))),
                ComponentRigging::for_test("duck", Some(json!({"b": "$$.bird"}))),
                ComponentRigging::for_test("elk", Some(json!({"d": "$$.duck"}))),
                ComponentRigging::for_test("fish", Some(json!({"e": "$$.elk"}))),
                ComponentRigging::for_test("goat", Some(json!({"f": "$$.fish"}))),
            ]
            .into_iter()
            .collect(),
        });

        let app_session = AppSession::from(app);
        let mut state = app_session.initialize().unwrap();

        state = state
            .step(Instruction::SetOutput {
                handle: ch("ant"),
                value: json!(0),
            })
            .unwrap();

        state = state
            .step(Instruction::SetOutput {
                handle: ch("bird"),
                value: json!(0),
            })
            .unwrap();

        state = state
            .step(Instruction::SetOutput {
                handle: ch("ant"),
                value: json!(1),
            })
            .unwrap();

        state = state
            .step(Instruction::SetOutputOverride {
                handle: ch("cat"),
                value: json!(0),
            })
            .unwrap();

        state = state
            .step(Instruction::SetInputOverride {
                handle: ch("duck"),
                value: json!({"b": "$$.bird", "c": "$$.cat"}),
            })
            .unwrap();

        state = state
            .step(Instruction::SetOutput {
                handle: ch("duck"),
                value: json!(0),
            })
            .unwrap();

        state = state
            .step(Instruction::SetInputOverride {
                handle: ch("elk"),
                value: json!({"d": "$$.duck"}),
            })
            .unwrap();

        state = state
            .step(Instruction::SetOutputOverride {
                handle: ch("fish"),
                value: json!(" ".repeat(5000)),
            })
            .unwrap();

        let view_model = to_view_model(state);

        let mut buffer = Vec::new();
        write_app_state(&mut buffer, &view_model).unwrap();
        let buffer_string = String::from_utf8(buffer).unwrap();
        println!("{}", buffer_string);

        let mut lines = buffer_string.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), 7);

        // We have to be careful when finding the index because we need the index
        // before any control characters. Finding the last occurrence of two spaces
        // does the trick.
        fn hash_start_index(s: &str) -> usize {
            let search_string = format!("  {}  ", COLUMN_CHAR);
            s.find(&search_string).unwrap() + search_string.len()
        }

        fn size_end_index(s: &str) -> usize {
            s.len()
            // let search_string = format!("  {}", COLUMN_CHAR);
            // s.rfind(&search_string).unwrap()
        }

        fn get_next_line<'a>(lines: &mut Vec<&'a str>) -> &'a str {
            let line = lines.remove(0);
            &line[hash_start_index(line)..size_end_index(line)]
        }

        // Input and output hashes match.
        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "{}44136fa3{} ➜ {}6b86b273{}  ┆  {} 2 bytes{} ➜ {} 1 byte{}",
                color::Fg(color::Blue),
                color::Fg(color::Reset),
                color::Fg(color::Green),
                color::Fg(color::Reset),
                color::Fg(color::Blue),
                color::Fg(color::Reset),
                color::Fg(color::Green),
                color::Fg(color::Reset),
            ),
        );

        // Input and output hashes do not match.
        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "{}015abd7f{} ! {}5feceb66{}  ┆  {} 7 bytes{} ! {} 1 byte{}",
                color::Fg(color::Blue),
                color::Fg(color::Reset),
                color::Fg(color::Red),
                color::Fg(color::Reset),
                color::Fg(color::Blue),
                color::Fg(color::Reset),
                color::Fg(color::Red),
                color::Fg(color::Reset),
            ),
        );

        // Output has been overridden.
        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "{}015abd7f{} ! {}{}5feceb66{}{}  ┆  {} 7 bytes{} ! {} {}1 byte{}{}",
                color::Fg(color::Blue),
                color::Fg(color::Reset),
                color::Fg(color::Blue),
                style::Underline,
                style::Reset,
                color::Fg(color::Reset),
                color::Fg(color::Blue),
                color::Fg(color::Reset),
                color::Fg(color::Blue),
                style::Underline,
                style::Reset,
                color::Fg(color::Reset),
            ),
        );

        // Input has been overridden but hash matches output hash.
        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "{}{}53779b51{}{} ➜ {}5feceb66{}  ┆  {}{}13 bytes{}{} ➜ {} 1 byte{}",
                color::Fg(color::Blue),
                style::Underline,
                style::Reset,
                color::Fg(color::Reset),
                color::Fg(color::Green),
                color::Fg(color::Reset),
                color::Fg(color::Blue),
                style::Underline,
                style::Reset,
                color::Fg(color::Reset),
                color::Fg(color::Green),
                color::Fg(color::Reset),
            ),
        );

        // Input has been overridden, no output.
        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "{}{}b852cecd{}{}             ┆  {} {}7 bytes{}{}          ",
                color::Fg(color::Blue),
                style::Underline,
                style::Reset,
                color::Fg(color::Reset),
                color::Fg(color::Blue),
                style::Underline,
                style::Reset,
                color::Fg(color::Reset),
            ),
        );

        // Output has been overridden, no input.
        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "         ! {}{}eadd1967{}{}  ┆           ! {}{}4.88 kb{}{}",
                color::Fg(color::Blue),
                style::Underline,
                style::Reset,
                color::Fg(color::Reset),
                color::Fg(color::Blue),
                style::Underline,
                style::Reset,
                color::Fg(color::Reset),
            ),
        );

        // Input, no output.
        assert_eq!(
            get_next_line(&mut lines),
            format!(
                "{}10c2cd2c{}             ┆  {} 4.89 kb{}          ",
                color::Fg(color::Blue),
                color::Fg(color::Reset),
                color::Fg(color::Blue),
                color::Fg(color::Reset),
            ),
        );
    }
}
