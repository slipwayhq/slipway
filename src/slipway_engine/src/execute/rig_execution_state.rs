use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use crate::{
    errors::RigError, Callouts, ComponentCache, ComponentHandle, ComponentInput, Immutable,
    Instruction, JsonMetadata, RigSession, SlipwayReference, PERMISSIONS_NONE_VEC,
};

use super::{
    component_execution_data::{
        CallChain, CalloutContext, ComponentExecutionContext, ComponentExecutionData,
    },
    component_runner::ComponentRunner,
    component_state::ComponentState,
    step::step,
};

#[derive(Clone)]
pub struct RigExecutionState<'rig, 'cache> {
    pub session: &'rig RigSession<'cache>,
    pub component_states: HashMap<&'rig ComponentHandle, ComponentState<'rig>>,
    pub valid_execution_order: Vec<&'rig ComponentHandle>,
    pub component_groups: Vec<HashSet<&'rig ComponentHandle>>,
}

impl<'rig, 'cache> RigExecutionState<'rig, 'cache> {
    pub fn step(
        &self,
        instruction: Instruction,
    ) -> Result<Immutable<RigExecutionState<'rig, 'cache>>, RigError> {
        step(self, instruction)
    }

    pub fn get_component_execution_data<'call, 'runners>(
        &self,
        handle: &'rig ComponentHandle,
        call_chain: Arc<CallChain<'rig>>,
        component_runners: &'runners [Box<dyn ComponentRunner>],
    ) -> Result<ComponentExecutionData<'call, 'rig, 'runners>, RigError>
    where
        'rig: 'call,
    {
        let component_state = self.get_component_state(handle)?;

        let input =
            component_state
                .execution_input
                .as_ref()
                .ok_or_else(|| RigError::StepFailed {
                    error: format!(
                        "Component {} has no execution input",
                        component_state.handle
                    ),
                })?;

        let permissions = component_state
            .rigging
            .permissions
            .as_ref()
            .unwrap_or(&PERMISSIONS_NONE_VEC);

        let call_chain = CallChain::new_child_arc(handle, Some(permissions), call_chain);

        let component_reference = &component_state.rigging.component;

        let outer_callouts = component_state.rigging.callouts.as_ref();

        get_component_execution_data(
            component_reference,
            self.session.component_cache,
            component_runners,
            call_chain,
            outer_callouts,
            Rc::clone(input),
        )
    }

    /// Internal because it returns a StepFailed error if the component does not exist,
    /// and so should only be used during a step.
    pub(super) fn get_component_state_mut(
        &mut self,
        handle: &ComponentHandle,
    ) -> Result<&mut ComponentState<'rig>, RigError> {
        let component_state =
            self.component_states
                .get_mut(handle)
                .ok_or(RigError::StepFailed {
                    error: format!(
                        "component \"{}\" does not exist in component states",
                        handle
                    ),
                })?;

        Ok(component_state)
    }

    /// Internal because it returns a StepFailed error if the component does not exist,
    /// and so should only be used during a step.
    pub(super) fn get_component_state(
        &self,
        handle: &ComponentHandle,
    ) -> Result<&ComponentState<'rig>, RigError> {
        let component_state = self
            .component_states
            .get(handle)
            .ok_or(RigError::StepFailed {
                error: format!(
                    "component \"{}\" does not exist in component states",
                    handle
                ),
            })?;

        Ok(component_state)
    }
}

pub(super) fn get_component_execution_data_for_callout<'call, 'rig, 'runners>(
    handle: &'rig ComponentHandle,
    input: serde_json::Value,
    execution_context: &ComponentExecutionContext<'call, 'rig, 'runners>,
) -> Result<ComponentExecutionData<'call, 'rig, 'runners>, RigError>
where
    'rig: 'call,
{
    let component_reference = execution_context
        .callout_context
        .get_component_reference_for_handle(handle)?;

    let component_cache = execution_context.component_cache;

    let call_chain =
        CallChain::new_child_arc(handle, None, Arc::clone(&execution_context.call_chain));

    // There are no outer callouts if we're already in a callout.
    let outer_callouts = None;

    let component_runners = execution_context.component_runners;

    let json_metadata = JsonMetadata::from_value(&input);

    let input = Rc::new(ComponentInput {
        value: input,
        json_metadata,
    });

    get_component_execution_data(
        component_reference,
        component_cache,
        component_runners,
        call_chain,
        outer_callouts,
        input,
    )
}

pub(super) fn get_component_execution_data<'call, 'rig, 'runners>(
    component_reference: &'rig SlipwayReference,
    component_cache: &'rig dyn ComponentCache,
    component_runners: &'runners [Box<dyn ComponentRunner>],
    call_chain: Arc<CallChain<'rig>>,
    outer_callouts: Option<&'rig Callouts>,
    input: Rc<ComponentInput>,
) -> Result<ComponentExecutionData<'call, 'rig, 'runners>, RigError>
where
    'rig: 'call,
{
    let primed_component = component_cache.get(component_reference);
    let component_definition = Arc::clone(&primed_component.definition);
    let files = Arc::clone(&primed_component.files);
    let component_callouts = primed_component.definition.callouts.as_ref();

    let callouts = get_callout_handle_to_reference_map(outer_callouts, component_callouts);

    let callout_context = CalloutContext::new(callouts);

    Ok(ComponentExecutionData::<'call, 'rig, 'runners> {
        input,
        context: ComponentExecutionContext {
            component_reference,
            component_definition,
            component_cache,
            component_runners,
            call_chain,
            files,
            callout_context,
        },
    })
}

fn get_callout_handle_to_reference_map<'rig>(
    outer_callouts: Option<&'rig Callouts>,
    component_callouts: Option<&'rig Callouts>,
) -> HashMap<&'rig ComponentHandle, &'rig SlipwayReference> {
    let mut callouts = HashMap::new();
    if let Some(callout_overrides) = &outer_callouts {
        for (handle, reference) in callout_overrides.iter() {
            callouts.insert(handle, reference);
        }
    }
    if let Some(component_callouts) = component_callouts {
        for (handle, reference) in component_callouts.iter() {
            callouts.entry(handle).or_insert(reference);
        }
    }
    callouts
}
