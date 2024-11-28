use errors::RunComponentError;
use thiserror::Error;

use slipway_engine::{
    errors::{ComponentLoadError, RigError},
    ComponentExecutionData, ComponentHandle, Immutable, Instruction, RigExecutionState, RigSession,
};

use crate::RunComponentResult;

pub mod errors;

#[derive(Error, Debug)]
pub enum RunError<THostError> {
    #[error("Rig error.\n{0}")]
    Rig(#[from] RigError),

    #[error("Component load failed during running.\n{0}")]
    ComponentLoadFailed(#[from] ComponentLoadError),

    #[error("No component runner was found for component \"{component_handle}\".")]
    ComponentRunnerNotFound { component_handle: ComponentHandle },

    #[error(
        "Run component failed for \"{component_handle}\" using \"{component_runner}\".\n{error:?}"
    )]
    RunComponentFailed {
        component_handle: ComponentHandle,
        component_runner: String,
        error: RunComponentError,
    },

    #[error("Host error.\n{0:?}")]
    HostError(THostError),
}

pub struct ComponentRunStartEvent<'rig> {
    pub component_handle: &'rig ComponentHandle,
}

pub struct ComponentRunEndEvent<'rig> {
    pub component_handle: &'rig ComponentHandle,
}

pub struct StateChangeEvent<'rig, 'state> {
    pub state: &'state Immutable<RigExecutionState<'rig>>,
    pub is_complete: bool,
}

pub trait RunEventHandler<'rig, THostError> {
    fn handle_component_run_start(
        &mut self,
        event: ComponentRunStartEvent<'rig>,
    ) -> Result<(), THostError>;

    fn handle_component_run_end(
        &mut self,
        event: ComponentRunEndEvent<'rig>,
    ) -> Result<(), THostError>;

    fn handle_state_changed<'state>(
        &mut self,
        event: StateChangeEvent<'rig, 'state>,
    ) -> Result<(), THostError>;
}

pub trait ComponentRunner<'rig> {
    fn identifier(&self) -> String;

    fn can_run_component(
        &self,
        handle: &ComponentHandle,
        execution_data: ComponentExecutionData<'rig>,
    ) -> Result<bool, ComponentLoadError>;

    fn run_component(
        &self,
        handle: &ComponentHandle,
        execution_data: ComponentExecutionData<'rig>,
    ) -> Result<RunComponentResult, RunComponentError>;
}

pub fn run_rig<'rig, THostError>(
    rig_session: &'rig RigSession,
    event_handler: &mut impl RunEventHandler<'rig, THostError>,
    component_runners: &[Box<dyn ComponentRunner<'rig>>],
) -> Result<Immutable<RigExecutionState<'rig>>, RunError<THostError>> {
    let mut state = rig_session.initialize()?;

    loop {
        let ready_components: Vec<&ComponentHandle> = state
            .component_states
            .iter()
            .filter_map(|(&handle, component_state)| {
                if component_state.execution_input.is_some() && component_state.output().is_none() {
                    Some(handle)
                } else {
                    None
                }
            })
            .collect();

        let is_complete = ready_components.is_empty();
        event_handler
            .handle_state_changed(StateChangeEvent {
                state: &state,
                is_complete,
            })
            .map_err(|e| RunError::HostError(e))?;

        if is_complete {
            break;
        }

        for handle in ready_components {
            event_handler
                .handle_component_run_start(ComponentRunStartEvent {
                    component_handle: handle,
                })
                .map_err(|e| RunError::HostError(e))?;

            let result = run_component(handle, &state, component_runners)?;

            event_handler
                .handle_component_run_end(ComponentRunEndEvent {
                    component_handle: handle,
                })
                .map_err(|e| RunError::HostError(e))?;

            state = state.step(Instruction::SetOutput {
                handle: handle.clone(),
                value: result.output,
                metadata: result.metadata,
            })?;
        }
    }

    Ok(state)
}

pub fn run_component<'rig, THostError>(
    handle: &ComponentHandle,
    state: &RigExecutionState<'rig>,
    component_runners: &[Box<dyn ComponentRunner<'rig>>],
) -> Result<RunComponentResult, RunError<THostError>> {
    let execution_data = state.get_component_execution_data(handle)?;

    let runner = get_component_runner(handle, execution_data.clone(), component_runners)?;

    runner
        .run_component(handle, execution_data)
        .map_err(|e| RunError::RunComponentFailed {
            component_handle: handle.clone(),
            component_runner: runner.identifier(),
            error: e,
        })
}

fn get_component_runner<'rig, 'r, THostError>(
    handle: &ComponentHandle,
    execution_data: ComponentExecutionData<'rig>,
    component_runners: &'r [Box<dyn ComponentRunner<'rig>>],
) -> Result<&'r dyn ComponentRunner<'rig>, RunError<THostError>> {
    for runner in component_runners {
        let can_run = runner.can_run_component(handle, execution_data.clone())?;
        if can_run {
            return Ok(runner.as_ref());
        }
    }

    Err(RunError::ComponentRunnerNotFound {
        component_handle: handle.clone(),
    })
}
