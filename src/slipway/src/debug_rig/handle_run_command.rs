use slipway_lib::{errors::RigError, ComponentHandle, Immutable, Instruction, RigExecutionState};
use slipway_wasmtime::run_component_wasm;
use tracing::debug;

use crate::SLIPWAY_COMPONENT_WASM_FILE_NAME;

use super::errors::SlipwayDebugError;

pub(super) fn handle_run_command<'rig>(
    handle: &'rig ComponentHandle,
    state: &RigExecutionState<'rig>,
) -> Result<Immutable<RigExecutionState<'rig>>, SlipwayDebugError> {
    let execution_data = state.get_component_execution_data(handle)?;

    let input = &execution_data.input.value;
    let wasm_bytes = execution_data
        .files
        .get_bin(SLIPWAY_COMPONENT_WASM_FILE_NAME)
        .map_err(|e| SlipwayDebugError::SlipwayError(RigError::ComponentLoadFailed(e)))?;

    let result = run_component_wasm(handle, input, wasm_bytes)?;

    debug!(
        "Prepare input: {:.2?}",
        result.metadata.prepare_input_duration
    );
    debug!(
        "Prepare component: {:.2?}",
        result.metadata.prepare_component_duration
    );
    debug!("Call component: {:.2?}", result.metadata.call_duration);
    debug!(
        "Process output: {:.2?}",
        result.metadata.process_output_duration
    );

    let new_state = state.step(Instruction::SetOutput {
        handle: handle.clone(),
        value: result.output,
        metadata: result.metadata,
    })?;

    Ok(new_state)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use slipway_lib::{
        utils::ch, BasicComponentsLoader, ComponentCache, ComponentRigging, Rig, RigSession,
        Rigging, SlipwayReference,
    };

    use common_test_utils::{get_slipway_test_component_path, SLIPWAY_TEST_COMPONENT_NAME};
    use slipway_wasmtime::WasmExecutionError;

    use super::*;

    fn create_rig(component_handle: &ComponentHandle, input: serde_json::Value) -> Rig {
        Rig::for_test(Rigging {
            components: [(
                component_handle.clone(),
                ComponentRigging::for_test_with_reference(
                    SlipwayReference::Local {
                        path: get_slipway_test_component_path(SLIPWAY_TEST_COMPONENT_NAME),
                    },
                    Some(input),
                ),
            )]
            .into_iter()
            .collect(),
        })
    }

    #[test]
    fn it_should_run_basic_component() {
        let handle = ch("test_component");
        let rig = create_rig(&handle, json!({ "type": "increment", "value": 42}));

        let component_cache =
            ComponentCache::primed(&rig, &BasicComponentsLoader::default()).unwrap();
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

        let component_cache =
            ComponentCache::primed(&rig, &BasicComponentsLoader::default()).unwrap();
        let rig_session = RigSession::new(rig, component_cache);
        let state = rig_session.initialize().unwrap();

        let maybe_state = handle_run_command(&handle, &state);

        match maybe_state {
            Err(SlipwayDebugError::WasmExecutionFailed(WasmExecutionError::RunCallFailed {
                source: Some(_),
            })) => {}
            Err(x) => panic!("Expected WasmExecutionFailed/RunCallFailed, got {:?}", x),
            Ok(_) => panic!("Expected WasmExecutionFailed/RunCallFailed, got result"),
        }
    }

    #[test]
    fn it_should_handle_component_that_errors() {
        let handle = ch("test_component");
        let rig = create_rig(&handle, json!({ "type": "error" }));

        let component_cache =
            ComponentCache::primed(&rig, &BasicComponentsLoader::default()).unwrap();
        let rig_session = RigSession::new(rig, component_cache);
        let state = rig_session.initialize().unwrap();

        let maybe_state = handle_run_command(&handle, &state);

        match maybe_state {
            Err(SlipwayDebugError::WasmExecutionFailed(
                WasmExecutionError::RunCallReturnedError { error },
            )) => {
                assert_eq!(error, "slipway-test-component-error");
            }
            Err(x) => panic!("Expected WasmExecutionFailed/RunCallFailed, got {:?}", x),
            Ok(_) => panic!("Expected WasmExecutionFailed/RunCallFailed, got result"),
        }
    }
}
