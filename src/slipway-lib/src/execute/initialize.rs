use std::collections::HashSet;

use crate::{errors::AppError, AppSession, ComponentState, Immutable};

use super::{evaluate_component_inputs::evaluate_component_inputs, AppExecutionState};

pub fn initialize(session: &AppSession) -> Result<Immutable<AppExecutionState>, AppError> {
    let component_states = session
        .app
        .rigging
        .components
        .iter()
        .map(|(handle, rigging)| {
            (
                handle,
                ComponentState {
                    handle,
                    rigging,
                    input_override: None,
                    output_override: None,
                    execution_input: None,
                    execution_output: None,
                    dependencies: HashSet::new(),
                },
            )
        })
        .collect();

    let state = AppExecutionState {
        session,
        component_states,
        valid_execution_order: Vec::new(),
        component_groups: Vec::new(),
    };

    let state = evaluate_component_inputs(state)?;

    Ok(Immutable::new(state))
}
