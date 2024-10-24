use std::path::PathBuf;

use slipway_lib::{ComponentHandle, RigExecutionState};

use crate::canvas::render_canvas;

use super::errors::SlipwayDebugError;

pub(super) fn handle_render_command<'rig>(
    handle: &'rig ComponentHandle,
    state: &RigExecutionState<'rig>,
    save_path: Option<PathBuf>,
) -> Result<(), SlipwayDebugError> {
    let component_state = state
        .component_states
        .get(&handle)
        .expect("Component should exist");

    let output = component_state.output().ok_or_else(|| {
        SlipwayDebugError::UserError(format!("Component {} has no output", handle))
    })?;

    render_canvas(handle, output, save_path).map_err(SlipwayDebugError::CanvasError)
}
