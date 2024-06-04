use serde::{Deserialize, Serialize};

use crate::{errors::AppError, AppExecutionState, ComponentHandle, Immutable};

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

pub(super) fn step<'app>(
    state: &AppExecutionState<'app>,
    instruction: Instruction,
) -> Result<Immutable<AppExecutionState<'app>>, AppError> {
    // The clone is inexpensive because the input and output JSON structures are reference counted.
    let state: AppExecutionState<'app> = state.clone();
    evaluate_component_inputs(evaluate_instruction(state, instruction)?).map(Immutable::new)
}
