use std::{io::Write, sync::Arc};

use slipway_engine::{CallChain, ComponentHandle, ComponentRunner, Immutable, RigExecutionState};
use slipway_host::{render_state::to_view_model::to_shortcuts, render_state::write_state};

use crate::json_editor::JsonEditor;

use super::{DebugCli, DebuggerCommand, errors::SlipwayDebugError};

pub(super) async fn handle_command<'rig, 'cache, W: Write>(
    w: &mut W,
    debug_cli: DebugCli,
    state: &RigExecutionState<'rig, 'cache>,
    json_editor: &impl JsonEditor,
    component_runners: &[Box<dyn ComponentRunner>],
    call_chain: Arc<CallChain<'rig>>,
) -> anyhow::Result<HandleCommandResult<'rig, 'cache>> {
    let result = match debug_cli.command {
        DebuggerCommand::Print {} => {
            write_state::<_, anyhow::Error>(w, state)?;
            HandleCommandResult::Continue(None)
        }
        DebuggerCommand::Run { handle } => {
            let handle = get_handle(&handle, state)?;
            let new_state = super::handle_run_command::handle_run_command(
                handle,
                state,
                component_runners,
                call_chain,
            )
            .await?;
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

        DebuggerCommand::Render { handle, save } => {
            let handle = get_handle(&handle, state)?;
            super::handle_render_command::handle_render_command(handle, state, save)?;
            writeln!(w)?;
            HandleCommandResult::Continue(None)
        }
        DebuggerCommand::Exit => HandleCommandResult::Exit,
    };

    Ok(result)
}

pub(super) enum HandleCommandResult<'rig, 'cache> {
    Continue(Option<Immutable<RigExecutionState<'rig, 'cache>>>),
    Exit,
}

#[allow(clippy::result_large_err)] // Ignoring this. Will fix once https://github.com/rust-lang/rust/issues/87121 is stable.
fn get_handle<'rig>(
    handle_str: &str,
    state: &RigExecutionState<'rig, '_>,
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
    use std::path::PathBuf;

    use common_macros::slipway_test_async;
    use serde_json::json;
    use slipway_engine::{
        BasicComponentCache, BasicComponentsLoaderBuilder, ComponentRigging, ComponentState, Rig,
        RigSession, Rigging, SlipwayReference, utils::ch,
    };

    use common_test_utils::{
        SLIPWAY_INCREMENT_COMPONENT_FOLDER_NAME, get_slipway_test_components_path,
        get_slipway_test_components_registry_url,
    };

    use crate::component_runners::get_component_runners;

    use super::*;

    #[slipway_test_async]
    async fn it_should_run_components_in_sequence() {
        let w = &mut std::io::sink();
        let (ch1, ch2, ch3) = component_handles();
        let rig_parts = get_rig_parts().await;
        let rig_session = RigSession::new_for_test(rig_parts.rig, &rig_parts.component_cache);
        let mut state = rig_session.initialize().unwrap();

        verify_state(&state, (true, false, false), (false, false, false)).await;
        assert_input_value(&state, &ch1, 1);

        // Run the first component.
        state =
            handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {}).await;
        verify_state(&state, (true, true, false), (true, false, false)).await;
        assert_input_value(&state, &ch2, 2);

        // Run the second component.
        state =
            handle_test_command(w, DebugCli::for_test("run ch2"), &state, &NoJsonEditor {}).await;
        verify_state(&state, (true, true, true), (true, true, false)).await;
        assert_input_value(&state, &ch3, 3);

        // Run the third component.
        state =
            handle_test_command(w, DebugCli::for_test("run ch3"), &state, &NoJsonEditor {}).await;
        verify_state(&state, (true, true, true), (true, true, true)).await;
        assert_input_value(&state, &ch3, 3);

        let ch3_state = component(&state, &ch3);
        assert_eq!(
            ch3_state.execution_output.as_ref().unwrap().value,
            json!({ "value": 4 })
        );
    }

    #[slipway_test_async]
    async fn rerunning_component_should_clear_output_override() {
        let w = &mut std::io::sink();
        let (_, ch2, _) = component_handles();
        let rig_parts = get_rig_parts().await;
        let rig_session = RigSession::new_for_test(rig_parts.rig, &rig_parts.component_cache);
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state =
            handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {}).await;
        assert_input_value(&state, &ch2, 2);

        // Override the output for ch1.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({ "value": 2 }));
                Ok(json!({ "value": 10 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("output ch1"), &state, &editor).await;
        assert_input_value(&state, &ch2, 10);

        // Re-running ch1 should clear output override.
        state =
            handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {}).await;
        assert_input_value(&state, &ch2, 2);
    }

    #[slipway_test_async]
    async fn running_output_clear_command_should_clear_output_override() {
        let w = &mut std::io::sink();
        let (_, ch2, _) = component_handles();
        let rig_parts = get_rig_parts().await;
        let rig_session = RigSession::new_for_test(rig_parts.rig, &rig_parts.component_cache);
        let mut state = rig_session.initialize().unwrap();

        verify_state(&state, (true, false, false), (false, false, false)).await;

        // Override the output for ch1.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({}));
                Ok(json!({ "value": 10 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("output ch1"), &state, &editor).await;
        assert_input_value(&state, &ch2, 10);

        verify_state(&state, (true, true, false), (false, false, false)).await;

        // Re-running ch1 should clear output override.
        state = handle_test_command(
            w,
            DebugCli::for_test("output ch1 --clear"),
            &state,
            &NoJsonEditor {},
        )
        .await;

        verify_state(&state, (true, false, false), (false, false, false)).await;
    }

    #[slipway_test_async]
    async fn running_output_command_twice_should_edit_overridden_output() {
        let w = &mut std::io::sink();
        let (_, ch2, _) = component_handles();
        let rig_parts = get_rig_parts().await;
        let rig_session = RigSession::new_for_test(rig_parts.rig, &rig_parts.component_cache);
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state =
            handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {}).await;
        assert_input_value(&state, &ch2, 2);

        // Override the output for ch1.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({ "value": 2 }));
                Ok(json!({ "value": 10 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("output ch1"), &state, &editor).await;
        assert_input_value(&state, &ch2, 10);

        // Override the output for ch1.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({ "value": 10 }));
                Ok(json!({ "value": 100 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("output ch1"), &state, &editor).await;
        assert_input_value(&state, &ch2, 100);
    }

    #[slipway_test_async]
    async fn running_output_command_without_changes_should_not_override_output() {
        let w = &mut std::io::sink();
        let (ch1, ch2, _) = component_handles();
        let rig_parts = get_rig_parts().await;
        let rig_session = RigSession::new_for_test(rig_parts.rig, &rig_parts.component_cache);
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state =
            handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {}).await;
        assert_input_value(&state, &ch2, 2);

        // Run output command but don't change output.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({ "value": 2 }));
                Ok(json!({ "value": 2 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("output ch1"), &state, &editor).await;
        assert_input_value(&state, &ch2, 2);

        let ch1_state = component(&state, &ch1);

        // Output override should not be set.
        assert!(&ch1_state.output_override.is_none());
    }

    #[slipway_test_async]
    async fn it_should_override_inputs() {
        let w = &mut std::io::sink();
        let (_, ch2, ch3) = component_handles();
        let rig_parts = get_rig_parts().await;
        let rig_session = RigSession::new_for_test(rig_parts.rig, &rig_parts.component_cache);
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state =
            handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {}).await;
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
        state = handle_test_command(w, DebugCli::for_test("input ch2"), &state, &editor).await;
        assert_input_value(&state, &ch2, 20);

        state =
            handle_test_command(w, DebugCli::for_test("run ch2"), &state, &NoJsonEditor {}).await;
        assert_input_value(&state, &ch3, 21);
    }

    #[slipway_test_async]
    async fn it_should_override_inputs_with_dependencies() {
        let w = &mut std::io::sink();
        let (_, ch2, ch3) = component_handles();
        let rig_parts = get_rig_parts().await;
        let rig_session = RigSession::new_for_test(rig_parts.rig, &rig_parts.component_cache);
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state =
            handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {}).await;
        verify_state(&state, (true, true, false), (true, false, false)).await;
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
        state = handle_test_command(w, DebugCli::for_test("input ch3"), &state, &editor).await;
        verify_state(&state, (true, true, true), (true, false, false)).await;
        assert_input_value(&state, &ch2, 2);
        assert_input_value(&state, &ch3, 2);
    }

    #[slipway_test_async]
    async fn running_input_clear_command_should_clear_input_override() {
        let w = &mut std::io::sink();
        let (ch1, _, _) = component_handles();
        let rig_parts = get_rig_parts().await;
        let rig_session = RigSession::new_for_test(rig_parts.rig, &rig_parts.component_cache);
        let mut state = rig_session.initialize().unwrap();

        assert_input_value(&state, &ch1, 1);

        // Run input command with new input.
        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({ "type": "increment", "value": 1 }));
                Ok(json!({ "type": "increment", "value": 10 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("input ch1"), &state, &editor).await;
        assert_input_value(&state, &ch1, 10);

        state = handle_test_command(
            w,
            DebugCli::for_test("input ch1 --clear"),
            &state,
            &NoJsonEditor {},
        )
        .await;
        assert_input_value(&state, &ch1, 1);
    }

    #[slipway_test_async]
    async fn running_input_command_twice_should_edit_overridden_input() {
        let w = &mut std::io::sink();
        let (_, ch2, _) = component_handles();
        let rig_parts = get_rig_parts().await;
        let rig_session = RigSession::new_for_test(rig_parts.rig, &rig_parts.component_cache);
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state =
            handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {}).await;
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
        state = handle_test_command(w, DebugCli::for_test("input ch2"), &state, &editor).await;
        assert_input_value(&state, &ch2, 20);

        let editor = MockJsonEditor {
            func: Box::new(|input| {
                assert_eq!(input, &json!({ "type": "increment", "value": 20 }));
                Ok(json!({ "type": "increment", "value": 200 }))
            }),
        };
        state = handle_test_command(w, DebugCli::for_test("input ch2"), &state, &editor).await;
        assert_input_value(&state, &ch2, 200);
    }

    #[slipway_test_async]
    async fn running_input_command_without_changes_should_not_override_input() {
        let w = &mut std::io::sink();
        let (_, ch2, _) = component_handles();
        let rig_parts = get_rig_parts().await;
        let rig_session = RigSession::new_for_test(rig_parts.rig, &rig_parts.component_cache);
        let mut state = rig_session.initialize().unwrap();

        // Run the first component.
        state =
            handle_test_command(w, DebugCli::for_test("run ch1"), &state, &NoJsonEditor {}).await;
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
        state = handle_test_command(w, DebugCli::for_test("input ch2"), &state, &editor).await;
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

    struct RigParts {
        rig: Rig,
        component_cache: BasicComponentCache,
    }

    async fn get_rig_parts() -> RigParts {
        let (ch1, ch2, ch3) = component_handles();

        let increment_reference = SlipwayReference::Local {
            path: PathBuf::from(SLIPWAY_INCREMENT_COMPONENT_FOLDER_NAME),
        };

        // ch1 -> ch2 -> ch3
        let rig = Rig::for_test(Rigging {
            components: [
                (
                    ch1.clone(),
                    ComponentRigging::for_test_with_reference(
                        increment_reference.clone(),
                        Some(json!({ "type": "increment", "value": 1 })),
                    ),
                ),
                (
                    ch2.clone(),
                    ComponentRigging::for_test_with_reference(
                        increment_reference.clone(),
                        Some(json!({ "type": "increment", "value": "$$.ch1.value" })),
                    ),
                ),
                (
                    ch3.clone(),
                    ComponentRigging::for_test_with_reference(
                        increment_reference.clone(),
                        Some(json!({ "type": "increment", "value": "$$.ch2.value" })),
                    ),
                ),
            ]
            .into_iter()
            .collect(),
        });

        let component_cache = BasicComponentCache::primed(
            &rig,
            &BasicComponentsLoaderBuilder::new()
                .registry_lookup_url(&get_slipway_test_components_registry_url())
                .local_base_directory(&get_slipway_test_components_path())
                .build(),
        )
        .await
        .unwrap();

        RigParts {
            rig,
            component_cache,
        }
    }

    async fn verify_state(
        state: &Immutable<RigExecutionState<'_, '_>>,
        inputs: (bool, bool, bool),
        outputs: (bool, bool, bool),
    ) {
        run_print(state).await;

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

    async fn handle_test_command<'rig, 'cache>(
        w: &mut std::io::Sink,
        debug_cli: DebugCli,
        state: &RigExecutionState<'rig, 'cache>,
        json_editor: &impl JsonEditor,
    ) -> Immutable<RigExecutionState<'rig, 'cache>> {
        let component_runners = get_component_runners();
        match handle_command(
            w,
            debug_cli,
            state,
            json_editor,
            &component_runners,
            CallChain::full_trust_arc(),
        )
        .await
        .unwrap()
        {
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

    async fn run_print(state: &Immutable<RigExecutionState<'_, '_>>) {
        let mut v = Vec::new();
        match handle_command(
            &mut v,
            DebugCli {
                command: DebuggerCommand::Print {},
            },
            state,
            &NoJsonEditor {},
            &[],
            CallChain::full_trust_arc(),
        )
        .await
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
