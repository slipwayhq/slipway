use std::sync::Arc;

use crate::{
    errors::{ComponentLoadError, RigError},
    CallChain, ComponentExecutionContext, ComponentHandle, RigExecutionState, RunMetadata,
};
use thiserror::Error;
use tracing::{span, Level};

use super::{
    component_execution_data::ComponentExecutionData,
    rig_execution_state::get_component_execution_data_for_callout,
    validate_component_io::{validate_component_io, ValidationData},
};

pub enum TryRunComponentResult {
    CannotRun,
    Ran { result: RunComponentResult },
}

pub struct RunComponentResult {
    pub output: serde_json::Value,
    pub metadata: RunMetadata,
}

#[derive(Error, Debug)]
pub enum RunComponentError {
    #[error("Execution error.\n{0}")]
    GenericError(#[from] anyhow::Error),

    #[error("{0}")]
    Other(String),

    #[error("{source}")]
    RunCallFailed { source: anyhow::Error },

    #[error("Component returned an error: {message}\nInner errors:\n{inner:#?}")]
    RunCallReturnedError { message: String, inner: Vec<String> },

    #[error("Serializing input JSON failed.\n{source}")]
    SerializeInputFailed { source: serde_json::Error },

    #[error("Deserializing output JSON failed.\n{source}")]
    DeserializeOutputFailed { source: serde_json::Error },

    #[error("Component load failed.\n{0}")]
    ComponentLoadFailed(#[from] ComponentLoadError),
}

pub trait ComponentRunner: Send + Sync {
    fn identifier(&self) -> String;

    fn run<'call>(
        &self,
        execution_data: &'call ComponentExecutionData<'call, '_, '_>,
    ) -> Result<TryRunComponentResult, RunComponentError>;
}

#[derive(Error, Debug)]
pub enum RunError<THostError> {
    #[error("Rig error.\n{0}")]
    Rig(#[from] RigError),

    #[error("Component load failed during running.\n{0}")]
    ComponentLoadFailed(#[from] ComponentLoadError),

    #[error("No component runner was found for component \"{component_handle}\".")]
    ComponentRunnerNotFound { component_handle: ComponentHandle },

    #[error(
        "Run component failed for component \"{component_handle}\" using \"{component_runner}\" runner.\n{error}"
    )]
    RunComponentFailed {
        component_handle: ComponentHandle,
        component_runner: String,
        error: RunComponentError,
    },

    #[error("Host error.\n{0:#?}")]
    HostError(THostError),
}

pub fn run_component<'rig, THostError>(
    handle: &ComponentHandle,
    state: &RigExecutionState<'rig, '_>,
    component_runners: &[Box<dyn ComponentRunner>],
    call_chain: Arc<CallChain<'rig>>,
) -> Result<RunComponentResult, RunError<THostError>> {
    let execution_data =
        state.get_component_execution_data(handle, call_chain, component_runners)?;

    run_component_inner(&execution_data)
}

pub fn run_component_callout<THostError>(
    handle: &ComponentHandle,
    input: serde_json::Value,
    execution_context: &ComponentExecutionContext,
) -> Result<RunComponentResult, RunError<THostError>> {
    let execution_data =
        get_component_execution_data_for_callout(handle, input, execution_context)?;

    validate_component_io(
        ValidationData::Input(&execution_data.input.value),
        Arc::clone(&execution_data.context.component_definition),
        handle,
    )?;

    let result = run_component_inner(&execution_data)?;

    validate_component_io(
        ValidationData::Output(&result.output),
        Arc::clone(&execution_data.context.component_definition),
        handle,
    )?;

    Ok(result)
}

fn run_component_inner<THostError>(
    execution_data: &ComponentExecutionData,
) -> Result<RunComponentResult, RunError<THostError>> {
    let handle = format!("{}", execution_data.context.component_handle());
    let _span_ = span!(Level::INFO, "component", %handle).entered();

    for runner in execution_data.context.component_runners {
        let result = runner
            .run(execution_data)
            .map_err(|e| RunError::RunComponentFailed {
                component_handle: execution_data.context.component_handle().clone(),
                component_runner: runner.identifier(),
                error: e,
            })?;

        match result {
            TryRunComponentResult::Ran { result } => return Ok(result),
            TryRunComponentResult::CannotRun => {}
        }
    }

    Err(RunError::ComponentRunnerNotFound {
        component_handle: execution_data.context.component_handle().clone(),
    })
}
