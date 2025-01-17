use std::{collections::HashMap, rc::Rc, sync::Arc};

use slipway_engine::{
    run_component, CallChain, ComponentExecutionContext, ComponentHandle, ComponentOutput,
    ComponentRunner, Immutable, Instruction, RigExecutionState, RigSession, RunComponentError,
    RunError,
};
use tracing::{span, Level};

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
    pub component_outputs: HashMap<&'rig ComponentHandle, Option<Rc<ComponentOutput>>>,
}

pub fn no_event_handler<'rig, 'cache>() -> impl RunEventHandler<'rig, 'cache, ()> {
    sink_run_event_handler::SinkRunEventHandler::new()
}

pub fn run_rig<'rig, 'cache, 'runners, THostError>(
    rig_session: &'rig RigSession<'cache>,
    event_handler: &mut impl RunEventHandler<'rig, 'cache, THostError>,
    component_runners: &'runners [Box<dyn ComponentRunner>],
    call_chain: Arc<CallChain<'rig>>,
) -> Result<RunRigResult<'rig>, RunError<THostError>>
where
    'cache: 'rig,
{
    // TODO:
    // for component_rigging in rig_session.rig.rigging.components.values() {
    //     let component_reference = &component_rigging.component;

    //     todo!("Test permissions for rig accessing component.");
    // }

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

            let result = run_component(handle, &state, component_runners, Arc::clone(&call_chain))?;

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

pub fn run_component_callout(
    execution_context: &ComponentExecutionContext,
    handle: ComponentHandle,
    input: serde_json::Value,
) -> Result<String, ComponentError> {
    let _span_ = span!(Level::INFO, "callout").entered();

    let handle_trail = || -> String {
        execution_context
            .call_chain
            .component_handle_trail_for(&handle)
    };

    let result =
        slipway_engine::run_component_callout::<anyhow::Error>(&handle, input, execution_context)
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

    serde_json::to_string(&result.output).map_err(|e| ComponentError {
        message: format!(
            "Failed to serialize output JSON for callout \"{}\"",
            handle_trail(),
        ),
        inner: vec![format!("{}", e)],
    })
}
