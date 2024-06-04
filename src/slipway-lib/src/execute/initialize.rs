use std::collections::HashSet;

use crate::{errors::AppError, AppExecutionState, AppSession, ComponentState, Immutable};

use super::evaluate_component_inputs::evaluate_component_inputs;

pub(super) fn initialize(session: &AppSession) -> Result<Immutable<AppExecutionState>, AppError> {
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

    prime_component_cache(session);

    let state = evaluate_component_inputs(state)?;

    Ok(Immutable::new(state))
}

fn prime_component_cache(session: &AppSession) {
    let distinct_component_references: HashSet<_> = session
        .app
        .rigging
        .components
        .values()
        .map(|v| &v.component)
        .collect();

    for component_reference in distinct_component_references {
        session
            .component_cache
            .borrow_mut()
            .prime_cache_for(component_reference);
    }
}
