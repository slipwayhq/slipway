use serde::{Deserialize, Serialize};

use crate::{errors::SlipwayError, ComponentHandle};

use super::{evaluate_component_inputs::evaluate_component_inputs, AppExecutionState};

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

pub fn step(
    state: AppExecutionState,
    instruction: Instruction,
) -> Result<AppExecutionState, SlipwayError> {
    evaluate_component_inputs(evaluate_instruction(state, instruction)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_have_tests() {
        todo!();
    }
}
