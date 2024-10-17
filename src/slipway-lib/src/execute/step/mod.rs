use serde::{Deserialize, Serialize};

use crate::{errors::RigError, ComponentHandle, Immutable, RigExecutionState};

use super::evaluate_component_inputs::evaluate_component_inputs;

mod evaluate_instruction;

use evaluate_instruction::evaluate_instruction;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "operation")]
#[serde(rename_all = "snake_case")]
pub enum Instruction {
    // Overrides the input of a component. The overridden input is validated against against the component's input schema.
    SetInputOverride {
        handle: ComponentHandle,
        value: serde_json::Value,
    },

    // Clears the input override of a component.
    ClearInputOverride {
        handle: ComponentHandle,
    },

    // Overrides the output of a component. The overridden output is not validated against
    // the component's output schema, but it is validated against any subsequent component's input schema.
    SetOutputOverride {
        handle: ComponentHandle,
        value: serde_json::Value,
    },

    // Clears the output override of a component.
    ClearOutputOverride {
        handle: ComponentHandle,
    },

    // Sets the output of a component.
    SetOutput {
        handle: ComponentHandle,
        value: serde_json::Value,
    },
}

pub(super) fn step<'rig>(
    state: &RigExecutionState<'rig>,
    instruction: Instruction,
) -> Result<Immutable<RigExecutionState<'rig>>, RigError> {
    // The clone is inexpensive because the input and output JSON structures are reference counted.
    let state: RigExecutionState<'rig> = state.clone();
    evaluate_component_inputs(evaluate_instruction(state, instruction)?).map(Immutable::new)
}
