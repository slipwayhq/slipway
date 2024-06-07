use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use crate::{
    errors::AppError, AppSession, ComponentHandle, ComponentInput, ComponentPermission, Immutable,
    Instruction,
};

use super::{component_state::ComponentState, step::step};

#[derive(Clone)]
pub struct AppExecutionState<'app> {
    pub session: &'app AppSession,
    pub component_states: HashMap<&'app ComponentHandle, ComponentState<'app>>,
    pub valid_execution_order: Vec<&'app ComponentHandle>,
    pub component_groups: Vec<HashSet<&'app ComponentHandle>>,
}

#[derive(Clone)]
pub struct ComponentExecutionData<'app> {
    pub wasm_bytes: Arc<Vec<u8>>,
    pub input: Rc<ComponentInput>,
    pub permissions: Option<&'app Vec<ComponentPermission>>,
}

impl<'app> AppExecutionState<'app> {
    pub fn step(
        &self,
        instruction: Instruction,
    ) -> Result<Immutable<AppExecutionState<'app>>, AppError> {
        step(self, instruction)
    }

    pub fn get_component_execution_data(
        &self,
        handle: &ComponentHandle,
    ) -> Result<ComponentExecutionData<'app>, AppError> {
        let component_state = self.get_component_state(handle)?;

        let execution_input = component_state.execution_input.as_ref().ok_or_else(|| {
            AppError::StepFailed(format!(
                "Component {} has no execution input",
                component_state.handle
            ))
        })?;

        let mut component_cache = self.session.component_cache.borrow_mut();
        let component_reference = &component_state.rigging.component;
        let maybe_component_wasm = component_cache.get_wasm(component_reference);

        match &maybe_component_wasm.value {
            Some(wasm_bytes) => {
                let wasm_bytes = Arc::clone(wasm_bytes);
                let input = Rc::clone(execution_input);
                let permissions = component_state.rigging.permissions.as_ref();
                Ok(ComponentExecutionData {
                    wasm_bytes,
                    input,
                    permissions,
                })
            }
            None => Err(AppError::ComponentWasmLoadFailed(
                component_state.handle.clone(),
                maybe_component_wasm.loader_failures.clone(),
            )),
        }
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
