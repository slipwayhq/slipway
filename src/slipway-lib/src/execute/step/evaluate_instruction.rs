use crate::{
    errors::SlipwayError,
    execute::{
        get_component_state_mut, AppExecutionState, ComponentInputOverride, ComponentOutput,
        ComponentOutputOverride,
    },
};

use super::Instruction;

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

            let input = component_state
                .execution_input
                .as_ref()
                .ok_or(SlipwayError::StepFailed(format!(
                "component {} cannot currently be executed, did you intend to override the output?",
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
