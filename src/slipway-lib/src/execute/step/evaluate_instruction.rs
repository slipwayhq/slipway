use std::rc::Rc;

use crate::{
    errors::AppError,
    execute::{
        primitives::JsonMetadata,
        validate_component_io::{validate_component_io, ValidationData},
        AppExecutionState, ComponentInputOverride, ComponentOutput, ComponentOutputOverride,
    },
};

use super::Instruction;

pub fn evaluate_instruction(
    state: AppExecutionState,
    instruction: Instruction,
) -> Result<AppExecutionState, AppError> {
    match instruction {
        Instruction::SetInputOverride { handle, value } => {
            let mut state = state;
            let component_state = state.get_component_state_mut(&handle)?;
            component_state.input_override = Some(Rc::new(ComponentInputOverride { value }));
            Ok(state)
        }
        Instruction::ClearInputOverride { handle } => {
            let mut state = state;
            let component_state = state.get_component_state_mut(&handle)?;
            component_state.input_override = None;
            Ok(state)
        }
        Instruction::SetOutputOverride { handle, value } => {
            let mut state = state;
            let component_state = state.get_component_state_mut(&handle)?;
            let metadata = JsonMetadata::from_value(&value);
            component_state.output_override =
                Some(Rc::new(ComponentOutputOverride { value, metadata }));
            Ok(state)
        }
        Instruction::ClearOutputOverride { handle } => {
            let mut state = state;
            let component_state = state.get_component_state_mut(&handle)?;
            component_state.output_override = None;
            Ok(state)
        }
        Instruction::SetOutput { handle, value } => {
            {
                let component_state = state.get_component_state(&handle)?;
                validate_component_io(
                    state.session,
                    component_state,
                    ValidationData::Output(&value),
                )?;
            }

            let mut state = state;
            let component_state = state.get_component_state_mut(&handle)?;

            let input = component_state
                .execution_input
                .as_ref()
                .ok_or(AppError::StepFailed(format!(
                "component {} cannot currently be executed, did you intend to override the output?",
                handle
            )))?;

            let metadata = JsonMetadata::from_value(&value);
            component_state.execution_output = Some(Rc::new(ComponentOutput {
                value,
                input_hash_used: input.metadata.hash.clone(),
                metadata,
            }));

            Ok(state)
        }
    }
}
