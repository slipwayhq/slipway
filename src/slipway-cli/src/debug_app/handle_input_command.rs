use slipway_lib::{AppExecutionState, ComponentHandle, Immutable, Instruction};

use super::{edit_json, errors::SlipwayDebugError};

pub(super) fn handle_input_command<'app>(
    handle: &'app ComponentHandle,
    state: &AppExecutionState<'app>,
) -> Result<Immutable<AppExecutionState<'app>>, SlipwayDebugError> {
    let component = state
        .component_states
        .get(&handle)
        .expect("Component should exist");

    let template = component.input().ok_or_else(|| {
        SlipwayDebugError::UserError(format!("Component {} has no input", handle))
    })?;

    let new_input = edit_json(template)?;

    let new_state = state.step(Instruction::SetInputOverride {
        handle: handle.clone(),
        value: new_input,
    })?;

    Ok(new_state)
}
