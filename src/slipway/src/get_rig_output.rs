use std::{str::FromStr, sync::Arc};

use anyhow::Context;
use slipway_engine::{ComponentHandle, ComponentOutput, Immutable, RigExecutionState};

pub(super) fn get_rig_output<'rig>(
    state: &Immutable<RigExecutionState<'rig, '_>>,
) -> Result<RigOutput<'rig>, anyhow::Error> {
    if state.component_states.len() == 1 {
        let (&handle, component_state) = state
            .component_states
            .iter()
            .next()
            .expect("Should be able to get the only component in a rig.");

        let output = component_state
            .execution_output
            .as_ref()
            .expect("The only component in a rig should have an output.");

        return Ok(RigOutput {
            handle,
            output: Arc::clone(output),
        });
    }

    const OUTPUT_COMPONENT_NAMES: [&str; 2] = ["render", "output"];

    for name in OUTPUT_COMPONENT_NAMES.iter() {
        let handle =
            &ComponentHandle::from_str(name).context("Failed to parse output component name.")?;

        if let Some(component_state) = state.component_states.get(handle) {
            if let Some(output) = component_state.execution_output.as_ref() {
                return Ok(RigOutput {
                    handle: component_state.handle,
                    output: Arc::clone(output),
                });
            }
        }
    }

    Err(anyhow::anyhow!(
        "Failed to identify output component. Expected handles are: {:?}",
        OUTPUT_COMPONENT_NAMES
    ))
}

pub(super) struct RigOutput<'rig> {
    pub handle: &'rig ComponentHandle,
    pub output: Arc<ComponentOutput>,
}
