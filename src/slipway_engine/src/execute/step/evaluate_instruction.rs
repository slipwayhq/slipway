use std::rc::Rc;

use crate::{
    errors::RigError,
    execute::{
        primitives::JsonMetadata,
        validate_component_io::{validate_component_io_from_session, ValidationData},
    },
    ComponentInputOverride, ComponentOutput, ComponentOutputOverride, RigExecutionState,
};

use super::Instruction;

pub(super) fn evaluate_instruction<'rig, 'cache>(
    state: RigExecutionState<'rig, 'cache>,
    instruction: Instruction,
) -> Result<RigExecutionState<'rig, 'cache>, RigError> {
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
            let json_metadata = JsonMetadata::from_value(&value);
            component_state.output_override = Some(Rc::new(ComponentOutputOverride {
                value,
                json_metadata,
            }));
            Ok(state)
        }
        Instruction::ClearOutputOverride { handle } => {
            let mut state = state;
            let component_state = state.get_component_state_mut(&handle)?;
            component_state.output_override = None;
            Ok(state)
        }
        Instruction::SetOutput {
            handle,
            value,
            metadata,
        } => {
            {
                let component_state = state.get_component_state(&handle)?;
                validate_component_io_from_session(
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

            let json_metadata = JsonMetadata::from_value(&value);
            component_state.output_override = None;
            component_state.execution_output = Some(Rc::new(ComponentOutput {
                value,
                input_hash_used: input.json_metadata.hash.clone(),
                json_metadata,
                run_metadata: metadata,
            }));

            Ok(state)
        }
    }
}
