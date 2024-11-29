use std::sync::Arc;

use slipway_engine::{ComponentHandle, Immutable, Instruction, PermissionChain, RigExecutionState};
use slipway_host::run::ComponentRunner;
use tracing::debug;

use super::errors::SlipwayDebugError;

pub(super) fn handle_run_command<'rig>(
    handle: &'rig ComponentHandle,
    state: &RigExecutionState<'rig>,
    component_runners: &[Box<dyn ComponentRunner<'rig>>],
    permission_chain: Arc<PermissionChain<'rig>>,
) -> Result<Immutable<RigExecutionState<'rig>>, SlipwayDebugError> {
    let result =
        slipway_host::run::run_component(handle, state, component_runners, permission_chain)?;

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
    use slipway_engine::{
        utils::ch, BasicComponentsLoader, ComponentCache, ComponentRigging, Rig, RigSession,
        Rigging, SlipwayReference,
    };
    use slipway_host::run::{errors::RunComponentError, RunError};

    use common_test_utils::{get_slipway_test_component_path, SLIPWAY_TEST_COMPONENT_NAME};
    use slipway_wasmtime_runner::WASMTIME_COMPONENT_RUNNER_IDENTIFIER;

    use crate::{component_runners::get_component_runners, host_error::HostError};

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
        let component_runners = get_component_runners();

        state = handle_run_command(
            &handle,
            &state,
            &component_runners,
            PermissionChain::full_trust_arc(),
        )
        .unwrap();

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
        let component_runners = get_component_runners();

        let maybe_state = handle_run_command(
            &handle,
            &state,
            &component_runners,
            PermissionChain::full_trust_arc(),
        );

        match maybe_state {
            Err(SlipwayDebugError::RunError(RunError::<HostError>::RunComponentFailed {
                component_handle: _,
                component_runner: _,
                error: RunComponentError::RunCallFailed { source: Some(_) },
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
        let component_runners = get_component_runners();

        let maybe_state = handle_run_command(
            &handle,
            &state,
            &component_runners,
            PermissionChain::full_trust_arc(),
        );

        match maybe_state {
            Err(SlipwayDebugError::RunError(RunError::<HostError>::RunComponentFailed {
                component_handle,
                component_runner,
                error: RunComponentError::RunCallReturnedError { error },
            })) => {
                assert_eq!(component_handle, handle);
                assert_eq!(component_runner, WASMTIME_COMPONENT_RUNNER_IDENTIFIER);
                assert_eq!(error, "slipway-test-component-error");
            }
            Err(x) => panic!("Expected WasmExecutionFailed/RunCallFailed, got {:?}", x),
            Ok(_) => panic!("Expected WasmExecutionFailed/RunCallFailed, got result"),
        }
    }
}
