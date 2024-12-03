use slipway_engine::{ComponentHandle, Immutable, Instruction, RigExecutionState};

use super::errors::SlipwayDebugError;

pub(super) fn handle_clear_output_command<'rig, 'cache>(
    handle: &'rig ComponentHandle,
    state: &RigExecutionState<'rig, 'cache>,
) -> Result<Immutable<RigExecutionState<'rig, 'cache>>, SlipwayDebugError> {
    let new_state = state.step(Instruction::ClearOutputOverride {
        handle: handle.clone(),
    })?;

    Ok(new_state)
}
