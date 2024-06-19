use slipway_lib::{AppExecutionState, ComponentHandle, Immutable, Instruction};

use super::{errors::SlipwayDebugError, json_editor::JsonEditor};

pub(super) fn handle_input_command<'app>(
    handle: &'app ComponentHandle,
    state: &AppExecutionState<'app>,
    json_editor: &impl JsonEditor,
) -> Result<Immutable<AppExecutionState<'app>>, SlipwayDebugError> {
    let component = state
        .component_states
        .get(&handle)
        .expect("Component should exist");

    let template = component.input().ok_or_else(|| {
        SlipwayDebugError::UserError(format!("Component {} has no input", handle))
    })?;

    let new_input = json_editor.edit(template)?;

    if new_input == *template {
        // Nothing changed, so don't set input override.
        return Ok(Immutable::new(state.clone()));
    }

    let new_state = state.step(Instruction::SetInputOverride {
        handle: handle.clone(),
        value: new_input,
    })?;

    Ok(new_state)
}
