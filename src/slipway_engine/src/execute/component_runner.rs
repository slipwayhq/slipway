use std::{path::Path, sync::Arc};

use crate::{
    CallChain, ComponentExecutionContext, ComponentFiles, ComponentHandle, RigExecutionState,
    RunMetadata, SlipwayReference,
    errors::{ComponentLoadError, RigError},
};
use async_trait::async_trait;
use thiserror::Error;
use tracing::{Instrument, info_span};

use super::{
    component_execution_data::ComponentExecutionData,
    rig_execution_state::get_component_execution_data_for_callout,
    validate_component_io::{ValidationData, validate_component_io},
};

pub enum TryAotCompileComponentResult {
    Compiled,
    CannotCompile,
}
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

#[async_trait(?Send)]
pub trait ComponentRunner: Send + Sync {
    fn identifier(&self) -> String;

    async fn aot_compile(
        &self,
        _component_reference: &SlipwayReference,
        _aot_path: &Path,
        _target: Option<&str>,
        _files: Arc<ComponentFiles>,
    ) -> Result<TryAotCompileComponentResult, RunComponentError> {
        Ok(TryAotCompileComponentResult::CannotCompile)
    }

    async fn run<'call>(
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

pub async fn run_component<'rig, THostError>(
    handle: &ComponentHandle,
    state: &RigExecutionState<'rig, '_>,
    component_runners: &[Box<dyn ComponentRunner>],
    call_chain: Arc<CallChain<'rig>>,
) -> Result<RunComponentResult, RunError<THostError>> {
    let execution_data =
        state.get_component_execution_data(handle, Arc::clone(&call_chain), component_runners)?;

    if state.session.run_record_enabled() {
        let component_state = state
            .component_states
            .get(handle)
            .expect("component state should exist");

        let input = component_state
            .execution_input
            .as_ref()
            .expect("input should exist");

        state.session.push_run_record(
            component_state.rigging.component.clone(),
            Arc::clone(&execution_data.context.call_chain),
            Arc::clone(input),
            component_state.rigging.callouts.clone(),
        );
    }

    run_component_inner(&execution_data).await
}

pub async fn run_component_callout<THostError>(
    handle: &ComponentHandle,
    input: serde_json::Value,
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
) -> Result<RunComponentResult, RunError<THostError>> {
    let execution_data =
        get_component_execution_data_for_callout(handle, input, execution_context)?;

    validate_component_io(
        ValidationData::Input(&execution_data.input.value),
        Arc::clone(&execution_data.context.component_definition),
        handle,
    )?;

    let result = run_component_inner(&execution_data).await?;

    validate_component_io(
        ValidationData::Output(&result.output),
        Arc::clone(&execution_data.context.component_definition),
        handle,
    )?;

    Ok(result)
}

async fn run_component_inner<THostError>(
    execution_data: &ComponentExecutionData<'_, '_, '_>,
) -> Result<RunComponentResult, RunError<THostError>> {
    let handle = format!("{}", execution_data.context.component_handle());

    for runner in execution_data.context.component_runners {
        let result = runner
            .run(execution_data)
            .instrument(info_span!("component", %handle))
            .await
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
