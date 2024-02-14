use serde::{Deserialize, Serialize};

use crate::{errors::SlipwayError, ComponentHandle};

use super::{
    evaluate_inputs::evaluate_inputs, get_component_state_mut, AppExecutionState,
    ComponentInputOverride, ComponentOutput, ComponentOutputOverride,
};

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
    evaluate_inputs(evaluate_instruction(state, instruction)?)
}

pub fn evaluate_instruction(
    state: AppExecutionState,
    instruction: Instruction,
) -> Result<AppExecutionState, SlipwayError> {
    match instruction {
        Instruction::SetInputOverride { handle, value } => {
            let mut state = state;
            let component_state = get_component_state_mut(&mut state, &handle)?;
            component_state.input_override = Some(ComponentInputOverride { value });
            Ok(state)
        }
        Instruction::ClearInputOverride { handle } => {
            let mut state = state;
            let component_state = get_component_state_mut(&mut state, &handle)?;
            component_state.input_override = None;
            Ok(state)
        }
        Instruction::SetOutputOverride { handle, value } => {
            let mut state = state;
            let component_state = get_component_state_mut(&mut state, &handle)?;
            component_state.output_override = Some(ComponentOutputOverride { value });
            Ok(state)
        }
        Instruction::ClearOutputOverride { handle } => {
            let mut state = state;
            let component_state = get_component_state_mut(&mut state, &handle)?;
            component_state.output_override = None;
            Ok(state)
        }
        Instruction::SetOutput { handle, value } => {
            let mut state = state;
            let component_state = get_component_state_mut(&mut state, &handle)?;

            let input =
                component_state
                    .execution_input
                    .as_ref()
                    .ok_or(SlipwayError::StepFailed(format!(
                        "component {:?} cannot be executed, did you intend to override the output?",
                        handle
                    )))?;

            component_state.execution_output = Some(ComponentOutput {
                value,
                input_hash_used: input.hash.clone(),
            });
            Ok(state)
        }
    }
}
