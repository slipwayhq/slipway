use serde_json::json;
use slipway_lib::{RigExecutionState, ComponentHandle, Immutable, Instruction};

use super::{errors::SlipwayDebugError, json_editor::JsonEditor};

pub(super) fn handle_output_command<'rig>(
    handle: &'rig ComponentHandle,
    state: &RigExecutionState<'rig>,
    json_editor: &impl JsonEditor,
) -> Result<Immutable<RigExecutionState<'rig>>, SlipwayDebugError> {
    let component = state
        .component_states
        .get(&handle)
        .expect("Component should exist");

    let default_template = json!({});
    let template = component.output().unwrap_or(&default_template);

    let new_output = json_editor.edit(template)?;

    if new_output == *template {
        // Nothing changed, so don't set output override.
        return Ok(Immutable::new(state.clone()));
    }

    let new_state = state.step(Instruction::SetOutputOverride {
        handle: handle.clone(),
        value: new_output,
    })?;

    Ok(new_state)
}
