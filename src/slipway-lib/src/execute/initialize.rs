use std::collections::{HashMap, HashSet};

use crate::{errors::SlipwayError, AppSession, ComponentState};

use super::{evaluate_component_inputs::evaluate_component_inputs, AppExecutionState};

pub fn initialize(session: &AppSession) -> Result<AppExecutionState, SlipwayError> {
    let component_states = session
        .app
        .rigging
        .components
        .keys()
        .map(|handle| {
            (
                handle,
                ComponentState {
                    handle,
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
        wasm_cache: HashMap::new(),
    };

    let state = evaluate_component_inputs(state)?;

    Ok(state)
}
