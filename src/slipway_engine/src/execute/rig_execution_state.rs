use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use crate::{
    errors::RigError, ComponentFiles, ComponentHandle, ComponentInput, ComponentPermission,
    Immutable, Instruction, RigSession,
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
    pub permissions: Option<&'rig Vec<ComponentPermission>>,
    pub files: Arc<dyn ComponentFiles>,
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

        self.get_component_execution_data_inner(component_state, input.clone())
    }

    pub fn get_component_execution_data_for_input(
        &self,
        handle: &ComponentHandle,
        input: Rc<ComponentInput>,
    ) -> Result<ComponentExecutionData<'rig>, RigError> {
        let component_state = self.get_component_state(handle)?;
        self.get_component_execution_data_inner(component_state, input)
    }

    fn get_component_execution_data_inner(
        &self,
        component_state: &ComponentState<'rig>,
        input: Rc<ComponentInput>,
    ) -> Result<ComponentExecutionData<'rig>, RigError> {
        let component_reference = &component_state.rigging.component;
        let files = self.session.component_cache.get_files(component_reference);

        let permissions = component_state.rigging.permissions.as_ref();
        Ok(ComponentExecutionData {
            files,
            input,
            permissions,
        })
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
