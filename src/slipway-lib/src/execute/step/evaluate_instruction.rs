use std::rc::Rc;

use crate::{
    errors::RigError,
    execute::{
        primitives::JsonMetadata,
        validate_component_io::{validate_component_io, ValidationData},
    },
    RigExecutionState, ComponentInputOverride, ComponentOutput, ComponentOutputOverride,
};

use super::Instruction;

pub(super) fn evaluate_instruction(
    state: RigExecutionState,
    instruction: Instruction,
) -> Result<RigExecutionState, RigError> {
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
                .ok_or(RigError::StepFailed {
                    error: format!(
                        "component {} cannot currently be executed, did you intend to override the output?",
                        handle
                    ),
                })?;

            let metadata = JsonMetadata::from_value(&value);
            component_state.output_override = None;
            component_state.execution_output = Some(Rc::new(ComponentOutput {
                value,
                input_hash_used: input.metadata.hash.clone(),
                metadata,
            }));

            Ok(state)
        }
    }
}
