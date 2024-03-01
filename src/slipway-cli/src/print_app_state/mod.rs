use std::fmt::Write;

use slipway_lib::AppExecutionState;

use termion::{color, style};

mod to_view_model;

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

pub(crate) fn format_app_state(state: &AppExecutionState<'_>) -> anyhow::Result<Vec<String>> {
    let mut result = Vec::new();
    let view_model = to_view_model::to_view_model(state);

    for group in view_model.groups.iter() {
        for component in group.components.iter() {
            let mut f = String::new();

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

            let component_color = match component.state.output() {
                Some(_) => ComponentColors::HasOutput,
                None => match component.state.execution_input {
                    Some(_) => ComponentColors::HasInput,
                    None => ComponentColors::Default,
                },
            };

            component_color.write_foreground(&mut f)?;

            write!(f, "• ")?;

            write!(f, "{}", color::Fg(color::Reset))?;

            write!(
                f,
                "{}{}{}{}{}",
                style::Underline,
                color::Fg(color::White),
                component.shortcut,
                style::Reset,
                skip_first_n_chars(&component.handle.0, component.shortcut.len())
            )?;

            write!(f, " ")?;

            if let Some(input) = &component.state.execution_input {
                let input_hash_string = format!("{}", input.hash)[..8].to_string();
                write!(
                    f,
                    "{}input={}{} ",
                    color::Fg(color::LightBlue),
                    input_hash_string,
                    color::Fg(color::Reset),
                )?;
            }

            if component.state.output().is_some() {
                let (color, output_hash) = {
                    let mut color = ComponentColors::HashesIgnored;
                    let mut output_hash = None;
                    if let Some(execution_output) = &component.state.execution_output {
                        output_hash = Some(&execution_output.value);
                        if let Some(execution_input) = &component.state.execution_input {
                            if execution_input.hash == execution_output.input_hash_used {
                                color = ComponentColors::HashesMatch;
                            } else {
                                color = ComponentColors::HashesDiffer
                            }
                        }
                    }
                    (color, output_hash)
                };

                color.write_foreground(&mut f)?;

                let output_hash_string = match output_hash {
                    Some(hash) => format!("{}", hash)[..8].to_string(),
                    None => "overridden".to_string(),
                };

                write!(f, "output={} ", output_hash_string)?;

                write!(f, "{}", color::Fg(color::Reset))?;
            }

            result.push(f);
        }
    }
    Ok(result)
}

fn skip_first_n_chars(s: &str, n: usize) -> &str {
    let char_pos = s
        .char_indices()
        .nth(n)
        .map(|(pos, _)| pos)
        .unwrap_or(s.len());
    &s[char_pos..]
}

#[derive(Debug)]
enum ComponentColors {
    Default,
    HasOutput,
    HasInput,
    HashesMatch,
    HashesDiffer,
    HashesIgnored,
}

impl ComponentColors {
    fn write_foreground<F: Write>(&self, f: &mut F) -> std::fmt::Result {
        match self {
            ComponentColors::Default => write!(f, "{}", color::Fg(color::White)),
            ComponentColors::HasOutput => write!(f, "{}", color::Fg(color::Green)),
            ComponentColors::HasInput => write!(f, "{}", color::Fg(color::Yellow)),
            ComponentColors::HashesMatch => write!(f, "{}", color::Fg(color::Green)),
            ComponentColors::HashesDiffer => write!(f, "{}", color::Fg(color::Red)),
            ComponentColors::HashesIgnored => write!(f, "{}", color::Fg(color::LightBlue)),
        }
    }
    fn write_background<F: Write>(&self, f: &mut F) -> std::fmt::Result {
        match self {
            ComponentColors::Default => write!(f, "{}", color::Bg(color::White)),
            ComponentColors::HasOutput => write!(f, "{}", color::Bg(color::Green)),
            ComponentColors::HasInput => write!(f, "{}", color::Bg(color::Yellow)),
            ComponentColors::HashesMatch => write!(f, "{}", color::Bg(color::Green)),
            ComponentColors::HashesDiffer => write!(f, "{}", color::Bg(color::Red)),
            ComponentColors::HashesIgnored => write!(f, "{}", color::Bg(color::LightBlue)),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use slipway_lib::{utils::ch, App, AppSession, ComponentRigging, Instruction, Rigging};

    use super::*;

    #[test]
    fn it_should_print() {
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
                value: json!(5),
            })
            .unwrap();

        // create a formatter that writes to a string

        let lines = format_app_state(&state).unwrap();

        for line in lines {
            println!("{}", line);
        }

        todo!();
    }
}
