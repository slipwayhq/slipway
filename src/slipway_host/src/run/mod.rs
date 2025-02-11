use std::{collections::HashMap, sync::Arc};

use slipway_engine::{
    errors::{ComponentLoadError, ComponentLoadErrorInner},
    run_component, CallChain, ComponentExecutionContext, ComponentHandle, ComponentOutput,
    ComponentRunner, Immutable, Instruction, RigExecutionState, RigSession, RunComponentError,
    RunError,
};
use tracing::{info_span, Instrument};

use crate::ComponentError;

pub mod sink_run_event_handler;

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
    pub component_outputs: HashMap<&'rig ComponentHandle, Option<Arc<ComponentOutput>>>,
}

pub fn no_event_handler<'rig, 'cache>() -> impl RunEventHandler<'rig, 'cache, ()> {
    sink_run_event_handler::SinkRunEventHandler::new()
}

pub async fn run_rig<'rig, 'cache, 'runners, THostError>(
    rig_session: &'rig RigSession<'cache>,
    event_handler: &mut impl RunEventHandler<'rig, 'cache, THostError>,
    component_runners: &'runners [Box<dyn ComponentRunner>],
    call_chain: Arc<CallChain<'rig>>,
) -> Result<RunRigResult<'rig>, RunError<THostError>>
where
    'cache: 'rig,
{
    check_rig_component_permissions(rig_session, &call_chain)?;

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

            let result =
                run_component(handle, &state, component_runners, Arc::clone(&call_chain)).await?;

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
            .map(|(&k, v)| (k, v.execution_output.as_ref().map(Arc::clone)))
            .collect(),
    })
}

fn check_rig_component_permissions<THostError>(
    rig_session: &RigSession<'_>,
    call_chain: &Arc<CallChain<'_>>,
) -> Result<(), RunError<THostError>> {
    for component_reference in rig_session.rigging_component_references() {
        crate::permissions::ensure_can_use_component_reference(
            component_reference,
            Arc::clone(call_chain),
        )
        .map_err(|e| {
            RunError::ComponentLoadFailed(ComponentLoadError {
                reference: Box::new(component_reference.clone()),
                error: ComponentLoadErrorInner::PermissionDenied {
                    message: e.message,
                    inner: e.inner,
                },
            })
        })?;
    }

    Ok(())
}

pub async fn run_component_callout(
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    handle: &ComponentHandle,
    input: serde_json::Value,
) -> Result<serde_json::Value, ComponentError> {
    let handle_trail = || -> String {
        execution_context
            .call_chain
            .component_handle_trail_for(handle)
    };

    let result =
        slipway_engine::run_component_callout::<anyhow::Error>(handle, input, execution_context)
            .instrument(info_span!("callout"))
            .await
            .map_err(|e| {
            let mut inner_errors = Vec::new();
            let message = format!("Failed to run component \"{}\"", handle_trail());

            if let RunError::RunComponentFailed {
                component_handle,
                component_runner,
                error: RunComponentError::RunCallReturnedError { message, inner },
            } = &e
            {
                inner_errors.push(format!(
                    "Run component failed for component \"{component_handle}\" using \"{component_runner}\" runner.",
                ));

                inner_errors.push(message.clone());
                inner_errors.extend(inner.clone());
            }
            else {
                inner_errors.push(format!("{}", e));
            }

            ComponentError {
                message,
                inner: inner_errors,
            }
        })?;

    Ok(result.output)
}
