use std::collections::HashSet;

use crate::{ComponentState, Immutable, RigExecutionState, RigSession, errors::RigError};

use super::evaluate_component_inputs::evaluate_component_inputs;

pub(super) fn initialize<'rig, 'cache>(
    session: &'rig RigSession<'cache>,
) -> Result<Immutable<RigExecutionState<'rig, 'cache>>, RigError> {
    let component_states = session
        .rig
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

    let state = RigExecutionState {
        session,
        component_states,
        valid_execution_order: Vec::new(),
        component_groups: Vec::new(),
    };

    let state = evaluate_component_inputs(state)?;

    Ok(Immutable::new(state))
}
