use std::collections::{HashMap, HashSet};

use crate::{errors::AppError, AppSession, ComponentHandle, Immutable, Instruction};

use super::{component_state::ComponentState, step::step};

#[derive(Clone)]
pub struct AppExecutionState<'app> {
    pub session: &'app AppSession,
    pub component_states: HashMap<&'app ComponentHandle, ComponentState<'app>>,
    pub valid_execution_order: Vec<&'app ComponentHandle>,
    pub component_groups: Vec<HashSet<&'app ComponentHandle>>,
}

impl<'app> AppExecutionState<'app> {
    pub fn step(
        &self,
        instruction: Instruction,
    ) -> Result<Immutable<AppExecutionState<'app>>, AppError> {
        step(self, instruction)
    }

    /// Internal because it returns a StepFailed error if the component does not exist,
    /// and so should only be used during a step.
    pub(super) fn get_component_state_mut(
        &mut self,
        handle: &ComponentHandle,
    ) -> Result<&mut ComponentState<'app>, AppError> {
        let component_state = self
            .component_states
            .get_mut(handle)
            .ok_or(AppError::StepFailed(format!(
                "component {:?} does not exist in component states",
                handle
            )))?;

        Ok(component_state)
    }

    /// Internal because it returns a StepFailed error if the component does not exist,
    /// and so should only be used during a step.
    pub(super) fn get_component_state(
        &self,
        handle: &ComponentHandle,
    ) -> Result<&ComponentState<'app>, AppError> {
        let component_state = self
            .component_states
            .get(handle)
            .ok_or(AppError::StepFailed(format!(
                "component {:?} does not exist in component states",
                handle
            )))?;

        Ok(component_state)
    }
}
