use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{ComponentHandle, Immutable, RigExecutionState, errors::RigError};

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
        metadata: RunMetadata,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct RunMetadata {
    pub prepare_input_duration: Duration,
    pub prepare_component_duration: Duration,
    pub call_duration: Duration,
    pub process_output_duration: Duration,
}

impl RunMetadata {
    pub fn overall_duration(&self) -> Duration {
        self.prepare_input_duration
            .checked_add(self.prepare_component_duration)
            .and_then(|d| d.checked_add(self.call_duration))
            .and_then(|d| d.checked_add(self.process_output_duration))
            .expect("Duration overflow")
    }

    pub fn add(&self, other: &RunMetadata) -> RunMetadata {
        RunMetadata {
            prepare_input_duration: self
                .prepare_input_duration
                .checked_add(other.prepare_input_duration)
                .expect("Duration overflow"),
            prepare_component_duration: self
                .prepare_component_duration
                .checked_add(other.prepare_component_duration)
                .expect("Duration overflow"),
            call_duration: self
                .call_duration
                .checked_add(other.call_duration)
                .expect("Duration overflow"),
            process_output_duration: self
                .process_output_duration
                .checked_add(other.process_output_duration)
                .expect("Duration overflow"),
        }
    }
}

pub(super) fn step<'rig, 'cache>(
    state: &RigExecutionState<'rig, 'cache>,
    instruction: Instruction,
) -> Result<Immutable<RigExecutionState<'rig, 'cache>>, RigError> {
    // The clone is inexpensive because the input and output JSON structures are reference counted.
    let state: RigExecutionState<'rig, 'cache> = state.clone();
    evaluate_component_inputs(evaluate_instruction(state, instruction)?).map(Immutable::new)
}
