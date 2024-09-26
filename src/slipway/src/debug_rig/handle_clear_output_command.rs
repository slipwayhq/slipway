use slipway_lib::{RigExecutionState, ComponentHandle, Immutable, Instruction};

use super::errors::SlipwayDebugError;

pub(super) fn handle_clear_output_command<'rig>(
    handle: &'rig ComponentHandle,
    state: &RigExecutionState<'rig>,
) -> Result<Immutable<RigExecutionState<'rig>>, SlipwayDebugError> {
    let new_state = state.step(Instruction::ClearOutputOverride {
        handle: handle.clone(),
    })?;

    Ok(new_state)
}
