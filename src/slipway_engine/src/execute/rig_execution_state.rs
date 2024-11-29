use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use crate::{
    errors::RigError, utils::ExpectWith, Callouts, ComponentCache, ComponentFiles, ComponentHandle,
    ComponentInput, ComponentPermission, ComponentRigging, Immutable, Instruction, PrimedComponent,
    RigSession, SlipwayReference, PERMISSIONS_FULL_TRUST_VEC, PERMISSIONS_NONE_VEC,
};

use super::{component_state::ComponentState, step::step};

#[derive(Clone)]
pub struct RigExecutionState<'rig> {
    pub session: &'rig RigSession,
    pub component_states: HashMap<&'rig ComponentHandle, ComponentState<'rig>>,
    pub valid_execution_order: Vec<&'rig ComponentHandle>,
    pub component_groups: Vec<HashSet<&'rig ComponentHandle>>,
}

#[derive(Clone)]
pub struct ComponentExecutionData<'rig> {
    pub input: Rc<ComponentInput>,
    pub context: ComponentExecutionContext<'rig>,
}

#[derive(Clone)]
pub struct ComponentExecutionContext<'rig> {
    pub permission_chain: Arc<PermissionChain<'rig>>,
    pub files: Arc<dyn ComponentFiles>,
    pub callout_context: CalloutContext<'rig>,
}

#[derive(Clone)]
pub struct PermissionChain<'rig> {
    current: &'rig Vec<ComponentPermission>,
    previous: Option<Arc<PermissionChain<'rig>>>,
}

impl<'rig> PermissionChain<'rig> {
    pub fn new(permissions: &'rig Vec<ComponentPermission>) -> PermissionChain<'rig> {
        PermissionChain {
            current: permissions,
            previous: None,
        }
    }

    pub fn full_trust() -> PermissionChain<'rig> {
        PermissionChain {
            current: &PERMISSIONS_FULL_TRUST_VEC,
            previous: None,
        }
    }

    pub fn full_trust_arc() -> Arc<PermissionChain<'rig>> {
        Arc::new(PermissionChain {
            current: &PERMISSIONS_FULL_TRUST_VEC,
            previous: None,
        })
    }
}

#[derive(Clone)]
pub struct CalloutContext<'rig> {
    callout_handle_to_reference: HashMap<&'rig ComponentHandle, &'rig SlipwayReference>,
    pub component_cache: &'rig ComponentCache,
}

impl<'rig> CalloutContext<'rig> {
    pub fn get_component_reference_for_handle(
        &self,
        handle: &ComponentHandle,
    ) -> &SlipwayReference {
        self.callout_handle_to_reference
            .get(handle)
            .expect_with(|| format!("Callout reference not found for handle {:?}", handle))
    }
}

impl<'rig> RigExecutionState<'rig> {
    pub fn step(
        &self,
        instruction: Instruction,
    ) -> Result<Immutable<RigExecutionState<'rig>>, RigError> {
        step(self, instruction)
    }

    pub fn get_component_execution_data(
        &self,
        handle: &ComponentHandle,
        permission_chain: Arc<PermissionChain<'rig>>,
    ) -> Result<ComponentExecutionData<'rig>, RigError> {
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

        let permission_chain = Arc::new(PermissionChain {
            current: permissions,
            previous: Some(permission_chain),
        });

        let component_reference = &component_state.rigging.component;

        let outer_callouts = &component_state.rigging.callouts;

        get_component_execution_data(
            component_reference,
            &self.session.component_cache,
            permission_chain,
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
                    error: format!("component {:?} does not exist in component states", handle),
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
                error: format!("component {:?} does not exist in component states", handle),
            })?;

        Ok(component_state)
    }
}

pub fn get_component_execution_data<'rig>(
    component_reference: &'rig SlipwayReference,
    component_cache: &'rig ComponentCache,
    permission_chain: Arc<PermissionChain<'rig>>,
    outer_callouts: &'rig Option<Callouts>,
    input: Rc<ComponentInput>,
) -> Result<ComponentExecutionData<'rig>, RigError> {
    let primed_component = component_cache.get(component_reference);
    let files = Arc::clone(&primed_component.files);
    let component_callouts = &primed_component.definition.callouts;

    let callouts = get_callout_handle_to_reference_map(outer_callouts, component_callouts);

    let callout_state = CalloutContext {
        callout_handle_to_reference: callouts,
        component_cache,
    };

    Ok(ComponentExecutionData {
        input,
        context: ComponentExecutionContext {
            permission_chain,
            files,
            callout_context: callout_state,
        },
    })
}

fn get_callout_handle_to_reference_map<'rig>(
    outer_callouts: &'rig Option<Callouts>,
    component_callouts: &'rig Option<Callouts>,
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
