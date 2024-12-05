use std::{collections::HashMap, rc::Rc, str::FromStr, sync::Arc};

use thiserror::Error;

use slipway_engine::{
    errors::{ComponentLoadError, RigError},
    get_component_execution_data_for_callout, ComponentExecutionContext, ComponentExecutionData,
    ComponentHandle, ComponentOutput, ComponentRunner, Immutable, Instruction, PermissionChain,
    RigExecutionState, RigSession, RunComponentError, RunComponentResult, TryRunComponentResult,
};

pub mod sink_run_event_handler;

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

pub struct StateChangeEvent<'rig, 'cache, 'state> {
    pub state: &'state Immutable<RigExecutionState<'rig, 'cache>>,
    pub is_complete: bool,
}

pub trait RunEventHandler<'rig, 'cache, THostError> {
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
        event: StateChangeEvent<'rig, 'cache, 'state>,
    ) -> Result<(), THostError>;
}

pub struct RunRigResult<'rig> {
    pub component_outputs: HashMap<&'rig ComponentHandle, Option<Rc<ComponentOutput>>>,
}

pub fn no_event_handler<'rig, 'cache>() -> impl RunEventHandler<'rig, 'cache, ()> {
    sink_run_event_handler::SinkRunEventHandler::new()
}

pub fn run_rig<'rig, 'cache, 'runners, THostError>(
    rig_session: &'rig RigSession<'cache>,
    event_handler: &mut impl RunEventHandler<'rig, 'cache, THostError>,
    component_runners: &'runners [Box<dyn ComponentRunner>],
    permission_chain: Arc<PermissionChain<'rig>>,
) -> Result<RunRigResult<'rig>, RunError<THostError>>
where
    'cache: 'rig,
{
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

            let result = run_component(
                handle,
                &state,
                component_runners,
                Arc::clone(&permission_chain),
            )?;

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

    Ok(RunRigResult {
        component_outputs: state
            .component_states
            .iter()
            .map(|(&k, v)| (k, v.execution_output.as_ref().map(Rc::clone)))
            .collect(),
    })
}

pub fn run_component<'rig, THostError>(
    handle: &ComponentHandle,
    state: &RigExecutionState<'rig, '_>,
    component_runners: &[Box<dyn ComponentRunner>],
    permission_chain: Arc<PermissionChain<'rig>>,
) -> Result<RunComponentResult, RunError<THostError>> {
    let execution_data =
        state.get_component_execution_data(handle, permission_chain, component_runners)?;

    run_component_inner(&execution_data)
}

pub fn run_component_callout_for_host(
    from_handle: &ComponentHandle,
    execution_context: &ComponentExecutionContext,
    handle: &str,
    input: &str,
) -> String {
    let handle = ComponentHandle::from_str(handle).unwrap_or_else(|e| {
        panic!(
            "Failed to parse component handle \"{}\" for callout from \"{}\":\n{}",
            handle, from_handle, e
        );
    });

    let input = serde_json::from_str(input).unwrap_or_else(|e| {
        panic!(
            "Failed to parse input JSON for callout from \"{}\":\n{}",
            from_handle, e
        );
    });

    let result = run_component_callout::<anyhow::Error>(&handle, input, execution_context)
        .unwrap_or_else(|e| {
            panic!(
                "Failed to run callout from \"{}\" to \"{}\":\n{}",
                from_handle, handle, e
            );
        });

    serde_json::to_string(&result.output).unwrap_or_else(|e| {
        panic!(
            "Failed to serialize output JSON for callout from \"{}\" to \"{}\":\n{}",
            from_handle, handle, e
        );
    })
}

pub fn run_component_callout<THostError>(
    handle: &ComponentHandle,
    input: serde_json::Value,
    execution_context: &ComponentExecutionContext,
) -> Result<RunComponentResult, RunError<THostError>> {
    let execution_data =
        get_component_execution_data_for_callout(handle, input, execution_context)?;

    run_component_inner(&execution_data)
}

fn run_component_inner<THostError>(
    execution_data: &ComponentExecutionData,
) -> Result<RunComponentResult, RunError<THostError>> {
    for runner in execution_data.context.component_runners {
        let result = runner
            .run(execution_data)
            .map_err(|e| RunError::RunComponentFailed {
                component_handle: execution_data.context.component_handle.clone(),
                component_runner: runner.identifier(),
                error: e,
            })?;

        match result {
            TryRunComponentResult::Ran { result } => return Ok(result),
            TryRunComponentResult::CannotRun => {}
        }
    }

    Err(RunError::ComponentRunnerNotFound {
        component_handle: execution_data.context.component_handle.clone(),
    })
}
