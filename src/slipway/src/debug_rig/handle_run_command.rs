use slipway_lib::{ComponentHandle, Immutable, Instruction, RigExecutionState};

use crate::run_component_wasm::run_component_wasm;

use super::errors::SlipwayDebugError;

pub(super) fn handle_run_command<'rig>(
    handle: &'rig ComponentHandle,
    state: &RigExecutionState<'rig>,
) -> Result<Immutable<RigExecutionState<'rig>>, SlipwayDebugError> {
    let execution_data = state.get_component_execution_data(handle)?;

    let output = run_component_wasm(execution_data, handle)?;

    let new_state = state.step(Instruction::SetOutput {
        handle: handle.clone(),
        value: output,
    })?;

    Ok(new_state)
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use serde_json::json;
    use slipway_lib::{
        utils::ch, BasicComponentsLoader, ComponentCache, ComponentRigging, Rig, RigSession,
        Rigging, SlipwayReference,
    };

    use crate::{
        run_component_wasm::errors::WasmExecutionError,
        test_utils::{find_ancestor_path, SLIPWAY_TEST_COMPONENT_PATH},
    };

    use super::*;

    fn create_rig(component_handle: &ComponentHandle, input: serde_json::Value) -> Rig {
        Rig::for_test(Rigging {
            components: [(
                component_handle.clone(),
                ComponentRigging {
                    component: SlipwayReference::Local {
                        path: find_ancestor_path(
                            PathBuf::from_str(SLIPWAY_TEST_COMPONENT_PATH).unwrap(),
                        ),
                    },
                    input: Some(input),
                    permissions: None,
                },
            )]
            .into_iter()
            .collect(),
        })
    }

    #[test]
    fn it_should_run_basic_component() {
        let handle = ch("test_component");
        let rig = create_rig(&handle, json!({ "type": "increment", "value": 42}));

        let component_cache = ComponentCache::primed(&rig, &BasicComponentsLoader::new()).unwrap();
        let rig_session = RigSession::new(rig, component_cache);
        let mut state = rig_session.initialize().unwrap();

        state = handle_run_command(&handle, &state).unwrap();

        let component_state = state
            .component_states
            .get(&handle)
            .expect("Component should exist");

        let maybe_execution_output = &component_state.execution_output;

        if let Some(execution_output) = maybe_execution_output {
            // The component will increment `value` by 1.
            assert_eq!(execution_output.value, json!({ "value": 43 }));
        } else {
            panic!("Component should have execution output");
        }
    }

    #[test]
    fn it_should_handle_component_that_panics() {
        let handle = ch("test_component");
        let rig = create_rig(&handle, json!({ "type": "panic" }));

        let component_cache = ComponentCache::primed(&rig, &BasicComponentsLoader::new()).unwrap();
        let rig_session = RigSession::new(rig, component_cache);
        let state = rig_session.initialize().unwrap();

        let maybe_state = handle_run_command(&handle, &state);

        match maybe_state {
            Err(SlipwayDebugError::WasmExecutionFailed(WasmExecutionError::StepCallFailed {
                message: stderr_string,
                source: Some(_),
            })) => {
                assert!(stderr_string.contains("slipway-test-component-panic"));
            }
            _ => panic!("Expected WasmExecutionError/StepCallFailed"),
        }
    }

    #[test]
    fn it_should_handle_component_that_errors() {
        let handle = ch("test_component");
        let rig = create_rig(&handle, json!({ "type": "stderr" }));

        let component_cache = ComponentCache::primed(&rig, &BasicComponentsLoader::new()).unwrap();
        let rig_session = RigSession::new(rig, component_cache);
        let state = rig_session.initialize().unwrap();

        let maybe_state = handle_run_command(&handle, &state);

        match maybe_state {
            Err(SlipwayDebugError::WasmExecutionFailed(WasmExecutionError::StepCallFailed {
                message: stderr_string,
                source: None,
            })) => {
                assert_eq!(stderr_string, "slipway-test-component-stderr");
            }
            _ => panic!("Expected WasmExecutionError/StepCallFailed"),
        }
    }
}
