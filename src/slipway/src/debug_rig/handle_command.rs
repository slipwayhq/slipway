use std::io::Write;

use slipway_lib::{RigExecutionState, ComponentHandle, Immutable};

use crate::to_view_model::to_shortcuts;

use super::{
    errors::SlipwayDebugError, json_editor::JsonEditor, print_state, DebugCli, DebuggerCommand,
};

pub(super) fn handle_command<'rig, W: Write>(
    w: &mut W,
    debug_cli: DebugCli,
    state: &RigExecutionState<'rig>,
    json_editor: &impl JsonEditor,
) -> anyhow::Result<HandleCommandResult<'rig>> {
    let result = match debug_cli.command {
        DebuggerCommand::Print {} => {
            print_state(w, state)?;
            HandleCommandResult::Continue(None)
        }
        DebuggerCommand::Run { handle } => {
            let handle = get_handle(&handle, state)?;
            let new_state = super::handle_run_command::handle_run_command(handle, state)?;
            HandleCommandResult::Continue(Some(new_state))
        }
        DebuggerCommand::Input { handle, clear } => {
            let handle = get_handle(&handle, state)?;

            match clear {
                true => {
                    let new_state = super::handle_clear_input_command::handle_clear_input_command(
                        handle, state,
                    )?;
                    HandleCommandResult::Continue(Some(new_state))
                }
                false => {
                    let new_state = super::handle_input_command::handle_input_command(
                        handle,
                        state,
                        json_editor,
                    )?;
                    HandleCommandResult::Continue(Some(new_state))
                }
            }
        }
        DebuggerCommand::Output { handle, clear } => {
            let handle = get_handle(&handle, state)?;

            match clear {
                true => {
                    let new_state =
                        super::handle_clear_output_command::handle_clear_output_command(
                            handle, state,
                        )?;
                    HandleCommandResult::Continue(Some(new_state))
                }
                false => {
                    let new_state = super::handle_output_command::handle_output_command(
                        handle,
                        state,
                        json_editor,
                    )?;
                    HandleCommandResult::Continue(Some(new_state))
                }
            }
        }
        DebuggerCommand::Exit => HandleCommandResult::Exit,
    };

    Ok(result)
}

pub(super) enum HandleCommandResult<'rig> {
    Continue(Option<Immutable<RigExecutionState<'rig>>>),
    Exit,
}

fn get_handle<'rig>(
    handle_str: &str,
    state: &RigExecutionState<'rig>,
) -> Result<&'rig ComponentHandle, SlipwayDebugError> {
    let shortcuts = to_shortcuts(state);

    if let Some(&handle) = shortcuts.get(handle_str) {
        return Ok(handle);
    }

    if let Some(&handle) = state
        .valid_execution_order
        .iter()
        .find(|&h| h.0 == handle_str)
    {
        return Ok(handle);
    }

    Err(SlipwayDebugError::UserError(format!(
        "No component found for handle or shortcut {}",
        handle_str
    )))
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use serde_json::json;
    use slipway_lib::{
        utils::ch, Rig, RigSession, BasicComponentsLoader, ComponentCache, ComponentRigging,
        ComponentState, Rigging, SlipwayReference,
    };

    use crate::test_utils::{find_ancestor_path, SLIPWAY_TEST_COMPONENT_PATH};

    use super::*;

    #[test]
    fn it_should_run_components_in_sequence() {
        let w = &mut std::io::sink();
        let (ch1, ch2, ch3) = component_handles();
        let rig_session = get_rig_session();
        let mut state = rig_session.initialize().unwrap();

        verify_state(&state, (true, false, false), (false, false, false));
        assert_input_value(&state, &ch1, 1);

        // Run the first component.
        state = handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {});
        verify_state(&state, (true, true, false), (true, false, false));
        assert_input_value(&state, &ch2, 2);

        // Run the second component.
        state = handle_test_command(w, DebugCli::for_test("run ch2"), &state, &NoJsonEditor {});
        verify_state(&state, (true, true, true), (true, true, false));
        assert_input_value(&state, &ch3, 3);

        // Run the third component.
        state = handle_test_command(w, DebugCli::for_test("run ch3"), &state, &NoJsonEditor {});
        verify_state(&state, (true, true, true), (true, true, true));
        assert_input_value(&state, &ch3, 3);

        let ch3_state = component(&state, &ch3);
        assert_eq!(
            ch3_state.execution_output.as_ref().unwrap().value,
            json!({ "value": 4 })
        );
    }

    #[test]
    fn rerunning_component_should_clear_output_override() {
        let w = &mut std::io::sink();
        let (_, ch2, _) = component_handles();
        let rig_session = get_rig_session();
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state = handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {});
        assert_input_value(&state, &ch2, 2);

        // Override the output for ch1.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({ "value": 2 }));
                Ok(json!({ "value": 10 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("output ch1"), &state, &editor);
        assert_input_value(&state, &ch2, 10);

        // Re-running ch1 should clear output override.
        state = handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {});
        assert_input_value(&state, &ch2, 2);
    }

    #[test]
    fn running_output_clear_command_should_clear_output_override() {
        let w = &mut std::io::sink();
        let (_, ch2, _) = component_handles();
        let rig_session = get_rig_session();
        let mut state = rig_session.initialize().unwrap();

        verify_state(&state, (true, false, false), (false, false, false));

        // Override the output for ch1.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({}));
                Ok(json!({ "value": 10 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("output ch1"), &state, &editor);
        assert_input_value(&state, &ch2, 10);

        verify_state(&state, (true, true, false), (false, false, false));

        // Re-running ch1 should clear output override.
        state = handle_test_command(
            w,
            DebugCli::for_test("output ch1 --clear"),
            &state,
            &NoJsonEditor {},
        );

        verify_state(&state, (true, false, false), (false, false, false));
    }

    #[test]
    fn running_output_command_twice_should_edit_overridden_output() {
        let w = &mut std::io::sink();
        let (_, ch2, _) = component_handles();
        let rig_session = get_rig_session();
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state = handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {});
        assert_input_value(&state, &ch2, 2);

        // Override the output for ch1.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({ "value": 2 }));
                Ok(json!({ "value": 10 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("output ch1"), &state, &editor);
        assert_input_value(&state, &ch2, 10);

        // Override the output for ch1.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({ "value": 10 }));
                Ok(json!({ "value": 100 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("output ch1"), &state, &editor);
        assert_input_value(&state, &ch2, 100);
    }

    #[test]
    fn running_output_command_without_changes_should_not_override_output() {
        let w = &mut std::io::sink();
        let (ch1, ch2, _) = component_handles();
        let rig_session = get_rig_session();
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state = handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {});
        assert_input_value(&state, &ch2, 2);

        // Run output command but don't change output.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({ "value": 2 }));
                Ok(json!({ "value": 2 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("output ch1"), &state, &editor);
        assert_input_value(&state, &ch2, 2);

        let ch1_state = component(&state, &ch1);

        // Output override should not be set.
        assert!(&ch1_state.output_override.is_none());
    }

    #[test]
    fn it_should_override_inputs() {
        let w = &mut std::io::sink();
        let (_, ch2, ch3) = component_handles();
        let rig_session = get_rig_session();
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state = handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {});
        assert_input_value(&state, &ch2, 2);

        // Run input command with new input.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(
                    input,
                    &json!({ "type": "increment", "value": "$$.ch1.value" })
                );
                Ok(json!({ "type": "increment", "value": 20 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("input ch2"), &state, &editor);
        assert_input_value(&state, &ch2, 20);

        state = handle_test_command(w, DebugCli::for_test("run ch2"), &state, &NoJsonEditor {});
        assert_input_value(&state, &ch3, 21);
    }

    #[test]
    fn it_should_override_inputs_with_dependencies() {
        let w = &mut std::io::sink();
        let (_, ch2, ch3) = component_handles();
        let rig_session = get_rig_session();
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state = handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {});
        verify_state(&state, (true, true, false), (true, false, false));
        assert_input_value(&state, &ch2, 2);

        // Run input command changing dependency of ch3 from ch2 to ch1.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(
                    input,
                    &json!({ "type": "increment", "value": "$$.ch2.value" })
                );
                Ok(json!({ "type": "increment", "value": "$$.ch1.value" }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("input ch3"), &state, &editor);
        verify_state(&state, (true, true, true), (true, false, false));
        assert_input_value(&state, &ch2, 2);
        assert_input_value(&state, &ch3, 2);
    }

    #[test]
    fn running_input_clear_command_should_clear_input_override() {
        let w = &mut std::io::sink();
        let (ch1, _, _) = component_handles();
        let rig_session = get_rig_session();
        let mut state = rig_session.initialize().unwrap();

        assert_input_value(&state, &ch1, 1);

        // Run input command with new input.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({ "type": "increment", "value": 1 }));
                Ok(json!({ "type": "increment", "value": 10 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("input ch1"), &state, &editor);
        assert_input_value(&state, &ch1, 10);

        state = handle_test_command(
            w,
            DebugCli::for_test("input ch1 --clear"),
            &state,
            &NoJsonEditor {},
        );
        assert_input_value(&state, &ch1, 1);
    }

    #[test]
    fn running_input_command_twice_should_edit_overridden_input() {
        let w = &mut std::io::sink();
        let (_, ch2, _) = component_handles();
        let rig_session = get_rig_session();
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state = handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {});
        assert_input_value(&state, &ch2, 2);

        // Run input command with new input.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(
                    input,
                    &json!({ "type": "increment", "value": "$$.ch1.value" })
                );
                Ok(json!({ "type": "increment", "value": 20 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("input ch2"), &state, &editor);
        assert_input_value(&state, &ch2, 20);

        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({ "type": "increment", "value": 20 }));
                Ok(json!({ "type": "increment", "value": 200 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("input ch2"), &state, &editor);
        assert_input_value(&state, &ch2, 200);
    }

    #[test]
    fn running_input_command_without_changes_should_not_override_input() {
        let w = &mut std::io::sink();
        let (_, ch2, _) = component_handles();
        let rig_session = get_rig_session();
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state = handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {});
        assert_input_value(&state, &ch2, 2);

        // Run input command but don't change input.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(
                    input,
                    &json!({ "type": "increment", "value": "$$.ch1.value" })
                );
                Ok(json!({ "type": "increment", "value": "$$.ch1.value" }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("input ch2"), &state, &editor);
        assert_input_value(&state, &ch2, 2);

        let ch2_state = component(&state, &ch2);

        // Input override should not be set.
        assert!(&ch2_state.input_override.is_none());
    }

    fn assert_input_value(
        state: &Immutable<RigExecutionState>,
        ch2: &ComponentHandle,
        expected_value: i32,
    ) {
        let ch2_state = component(state, ch2);
        assert_eq!(
            ch2_state.execution_input.as_ref().unwrap().value,
            json!({ "type": "increment", "value": expected_value })
        );
    }

    fn component_handles() -> (ComponentHandle, ComponentHandle, ComponentHandle) {
        (ch("ch1"), ch("ch2"), ch("ch3"))
    }

    fn get_rig_session() -> RigSession {
        let (ch1, ch2, ch3) = component_handles();

        let increment_reference = SlipwayReference::Local {
            path: find_ancestor_path(PathBuf::from_str(SLIPWAY_TEST_COMPONENT_PATH).unwrap()),
        };

        // ch1 -> ch2 -> ch3
        let rig = Rig::for_test(Rigging {
            components: [
                (
                    ch1.clone(),
                    ComponentRigging {
                        component: increment_reference.clone(),
                        input: Some(json!({ "type": "increment", "value": 1 })),
                        permissions: None,
                    },
                ),
                (
                    ch2.clone(),
                    ComponentRigging {
                        component: increment_reference.clone(),
                        input: Some(json!({ "type": "increment", "value": "$$.ch1.value" })),
                        permissions: None,
                    },
                ),
                (
                    ch3.clone(),
                    ComponentRigging {
                        component: increment_reference.clone(),
                        input: Some(json!({ "type": "increment", "value": "$$.ch2.value" })),
                        permissions: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        });

        let component_cache = ComponentCache::primed(&rig, &BasicComponentsLoader::new()).unwrap();
        RigSession::new(rig, component_cache)
    }

    fn verify_state(
        state: &Immutable<RigExecutionState>,
        inputs: (bool, bool, bool),
        outputs: (bool, bool, bool),
    ) {
        run_print(state);

        let ch1 = ch("ch1");
        let ch2 = ch("ch2");
        let ch3 = ch("ch3");

        assert_eq!(component(state, &ch1).execution_input.is_some(), inputs.0);
        assert_eq!(component(state, &ch2).execution_input.is_some(), inputs.1);
        assert_eq!(component(state, &ch3).execution_input.is_some(), inputs.2);

        assert_eq!(component(state, &ch1).execution_output.is_some(), outputs.0);
        assert_eq!(component(state, &ch2).execution_output.is_some(), outputs.1);
        assert_eq!(component(state, &ch3).execution_output.is_some(), outputs.2);
    }

    fn handle_test_command<'rig>(
        w: &mut std::io::Sink,
        debug_cli: DebugCli,
        state: &RigExecutionState<'rig>,
        json_editor: &impl JsonEditor,
    ) -> Immutable<RigExecutionState<'rig>> {
        match handle_command(w, debug_cli, state, json_editor).unwrap() {
            HandleCommandResult::Continue(Some(state)) => state,
            _ => panic!("Expected Continue"),
        }
    }

    fn component<'rig>(
        state: &'rig Immutable<RigExecutionState>,
        handle: &'rig ComponentHandle,
    ) -> &'rig ComponentState<'rig> {
        state.component_states.get(handle).unwrap()
    }

    fn run_print(state: &Immutable<RigExecutionState>) {
        let mut v = Vec::new();
        match handle_command(
            &mut v,
            DebugCli {
                command: DebuggerCommand::Print {},
            },
            state,
            &NoJsonEditor {},
        )
        .unwrap()
        {
            HandleCommandResult::Continue(None) => {}
            _ => panic!("Expected Continue"),
        };

        let s = String::from_utf8(v).unwrap();
        println!("{}", s);
        assert!(s.contains("bytes"));
    }

    type MockJsonEditorFn =
        dyn Fn(&serde_json::Value) -> Result<serde_json::Value, SlipwayDebugError>;

    struct MockJsonEditor {
        func: Box<MockJsonEditorFn>,
    }

    impl JsonEditor for MockJsonEditor {
        fn edit(
            &self,
            _initial: &serde_json::Value,
        ) -> Result<serde_json::Value, SlipwayDebugError> {
            (self.func)(_initial)
        }
    }

    struct NoJsonEditor {}

    impl JsonEditor for NoJsonEditor {
        fn edit(
            &self,
            _initial: &serde_json::Value,
        ) -> Result<serde_json::Value, SlipwayDebugError> {
            unimplemented!("NoJsonEditor should not be called")
        }
    }
}
