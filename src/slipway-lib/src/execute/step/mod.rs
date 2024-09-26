use serde::{Deserialize, Serialize};

use crate::{errors::RigError, RigExecutionState, ComponentHandle, Immutable};

use super::evaluate_component_inputs::evaluate_component_inputs;

mod evaluate_instruction;

use evaluate_instruction::evaluate_instruction;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "operation")]
#[serde(rename_all = "snake_case")]
pub enum Instruction {
    SetInputOverride {
        handle: ComponentHandle,
        value: serde_json::Value,
    },
    ClearInputOverride {
        handle: ComponentHandle,
    },
    SetOutputOverride {
        handle: ComponentHandle,
        value: serde_json::Value,
    },
    ClearOutputOverride {
        handle: ComponentHandle,
    },
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
