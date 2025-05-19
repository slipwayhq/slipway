use slipway_engine::{ComponentHandle, Immutable, Instruction, RigExecutionState};

use super::errors::SlipwayDebugError;

#[allow(clippy::result_large_err)] // Ignoring this. Will fix once https://github.com/rust-lang/rust/issues/87121 is stable.
pub(super) fn handle_clear_input_command<'rig, 'cache>(
    handle: &'rig ComponentHandle,
    state: &RigExecutionState<'rig, 'cache>,
) -> Result<Immutable<RigExecutionState<'rig, 'cache>>, SlipwayDebugError> {
    let new_state = state.step(Instruction::ClearInputOverride {
        handle: handle.clone(),
    })?;

    Ok(new_state)
}
