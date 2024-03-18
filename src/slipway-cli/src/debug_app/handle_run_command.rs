use slipway_lib::{AppExecutionState, ComponentHandle, Immutable, Instruction};

use super::{edit_json, errors::SlipwayDebugError};

pub(super) fn handle_run_command<'app>(
    handle: &'app ComponentHandle,
    state: &AppExecutionState<'app>,
) -> Result<Immutable<AppExecutionState<'app>>, SlipwayDebugError> {
    let component = state
        .component_states
        .get(&handle)
        .expect("Component should exist");

    let execution_input = component.execution_input.as_ref().ok_or_else(|| {
        SlipwayDebugError::UserError(format!("Component {} has no execution input", handle))
    })?;

    let execution_input = &execution_input.value;

    // TODO: Execute WASM
    let new_input = edit_json(execution_input)?;

    let new_state = state.step(Instruction::SetOutput {
        handle: handle.clone(),
        value: new_input,
    })?;

    Ok(new_state)
}
