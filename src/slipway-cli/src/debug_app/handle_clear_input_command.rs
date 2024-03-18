use slipway_lib::{AppExecutionState, ComponentHandle, Immutable, Instruction};

use super::errors::SlipwayDebugError;

pub(super) fn handle_clear_input_command<'app>(
    handle: &'app ComponentHandle,
    state: &AppExecutionState<'app>,
) -> Result<Immutable<AppExecutionState<'app>>, SlipwayDebugError> {
    let new_state = state.step(Instruction::ClearInputOverride {
        handle: handle.clone(),
    })?;

    Ok(new_state)
}
