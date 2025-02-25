use std::sync::Arc;

use slipway_engine::{
    CallChain, ComponentHandle, ComponentRunner, Immutable, Instruction, RigExecutionState,
};
use tracing::debug;

use super::errors::SlipwayDebugError;

pub(super) async fn handle_run_command<'rig, 'cache>(
    handle: &'rig ComponentHandle,
    state: &RigExecutionState<'rig, 'cache>,
    component_runners: &[Box<dyn ComponentRunner>],
    call_chain: Arc<CallChain<'rig>>,
) -> Result<Immutable<RigExecutionState<'rig, 'cache>>, SlipwayDebugError> {
    let result =
        slipway_engine::run_component(handle, state, component_runners, call_chain).await?;

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
    use std::path::PathBuf;

    use common_macros::slipway_test_async;
    use serde_json::json;
    use slipway_engine::{
        utils::ch, BasicComponentCache, BasicComponentsLoader, BasicComponentsLoaderBuilder,
        ComponentRigging, Rig, RigSession, Rigging, RunComponentError, RunError, SlipwayReference,
    };

    use common_test_utils::{
        get_slipway_test_components_path, SLIPWAY_INCREMENT_COMPONENT_FOLDER_NAME,
    };
    use slipway_wasmtime_runner::WASMTIME_COMPONENT_RUNNER_IDENTIFIER;

    use crate::component_runners::get_component_runners;

    use super::*;

    fn create_rig(component_handle: &ComponentHandle, input: serde_json::Value) -> Rig {
        Rig::for_test(Rigging {
            components: [(
                component_handle.clone(),
                ComponentRigging::for_test_with_reference(
                    SlipwayReference::Local {
                        path: PathBuf::from(SLIPWAY_INCREMENT_COMPONENT_FOLDER_NAME),
                    },
                    Some(input),
                ),
            )]
            .into_iter()
            .collect(),
        })
    }

    fn create_components_loader() -> BasicComponentsLoader {
        BasicComponentsLoaderBuilder::new()
            .local_base_directory(&get_slipway_test_components_path())
            .build()
    }

    #[slipway_test_async]
    async fn it_should_run_basic_component() {
        let handle = ch("test_component");
        let rig = create_rig(&handle, json!({ "type": "increment", "value": 42}));

        let component_cache = BasicComponentCache::primed(&rig, &create_components_loader())
            .await
            .unwrap();
        let rig_session = RigSession::new(rig, &component_cache);
        let mut state = rig_session.initialize().unwrap();
        let component_runners = get_component_runners();

        state = handle_run_command(
            &handle,
            &state,
            &component_runners,
            CallChain::full_trust_arc(),
        )
        .await
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

    #[slipway_test_async]
    async fn it_should_handle_component_that_panics() {
        let handle = ch("test_component");
        let rig = create_rig(&handle, json!({ "type": "panic" }));

        let component_cache = BasicComponentCache::primed(&rig, &create_components_loader())
            .await
            .unwrap();
        let rig_session = RigSession::new(rig, &component_cache);
        let state = rig_session.initialize().unwrap();
        let component_runners = get_component_runners();

        let maybe_state = handle_run_command(
            &handle,
            &state,
            &component_runners,
            CallChain::full_trust_arc(),
        )
        .await;

        match maybe_state {
            Err(SlipwayDebugError::RunError(RunError::RunComponentFailed {
                component_handle: _,
                component_runner: _,
                error: RunComponentError::RunCallFailed { source: _ },
            })) => {}
            Err(x) => panic!("Expected WasmExecutionFailed/RunCallFailed, got {}", x),
            Ok(_) => panic!("Expected WasmExecutionFailed/RunCallFailed, got result"),
        }
    }

    #[slipway_test_async]
    async fn it_should_handle_component_that_errors() {
        let handle = ch("test_component");
        let rig = create_rig(&handle, json!({ "type": "error" }));

        let component_cache = BasicComponentCache::primed(&rig, &create_components_loader())
            .await
            .unwrap();
        let rig_session = RigSession::new(rig, &component_cache);
        let state = rig_session.initialize().unwrap();
        let component_runners = get_component_runners();

        let maybe_state = handle_run_command(
            &handle,
            &state,
            &component_runners,
            CallChain::full_trust_arc(),
        )
        .await;

        match maybe_state {
            Err(SlipwayDebugError::RunError(RunError::RunComponentFailed {
                component_handle,
                component_runner,
                error:
                    RunComponentError::RunCallReturnedError {
                        message: error,
                        inner,
                    },
            })) => {
                assert_eq!(component_handle, handle);
                assert_eq!(component_runner, WASMTIME_COMPONENT_RUNNER_IDENTIFIER);
                assert_eq!(error, "slipway-increment-component-error");
                assert_eq!(inner.len(), 0);
            }
            Err(x) => panic!("Expected WasmExecutionFailed/RunCallFailed, got {}", x),
            Ok(_) => panic!("Expected WasmExecutionFailed/RunCallFailed, got result"),
        }
    }
}
