use std::path::PathBuf;

use slipway_engine::{ComponentHandle, RigExecutionState};

use crate::canvas::render_canvas;

use super::errors::SlipwayDebugError;

#[allow(clippy::result_large_err)] // Ignoring this. Will fix once https://github.com/rust-lang/rust/issues/87121 is stable.
pub(super) fn handle_render_command<'rig>(
    handle: &'rig ComponentHandle,
    state: &RigExecutionState<'rig, '_>,
    save_path: Option<PathBuf>,
) -> Result<(), SlipwayDebugError> {
    let component_state = state
        .component_states
        .get(&handle)
        .expect("Component should exist");

    let output = component_state.output().ok_or_else(|| {
        SlipwayDebugError::UserError(format!("Component {} has no output", handle))
    })?;

    if let Some(save_path) = save_path.as_ref() {
        if let Some(parent) = save_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                SlipwayDebugError::UserError(format!(
                    "Failed to create directory for save path: {}",
                    error
                ))
            })?;
        }
    }

    render_canvas(handle, output, save_path.as_deref()).map_err(SlipwayDebugError::CanvasError)
}
