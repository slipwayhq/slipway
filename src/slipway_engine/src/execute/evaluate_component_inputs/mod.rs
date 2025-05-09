use crate::{ComponentHandle, ComponentInput, RigExecutionState, errors::RigError};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use self::{
    extract_dependencies_from_json_path_strings::ExtractDependencies,
    find_json_path_strings::FoundJsonPathString,
};

use super::{
    topological_sort::sort_and_group,
    validate_component_io::{ValidationData, validate_component_io_from_session},
};

mod evaluate_input;
mod extract_dependencies_from_json_path_strings;
mod find_json_path_strings;
mod map_dependencies_to_rig_handles;
mod simple_json_path;

const RIGGING_KEY: &str = "rigging";
const RIG_CONTEXT_KEY: &str = "context";
const INPUT_KEY: &str = "input";
const OUTPUT_KEY: &str = "output";

pub(super) fn evaluate_component_inputs<'rig, 'cache>(
    state: RigExecutionState<'rig, 'cache>,
) -> Result<RigExecutionState<'rig, 'cache>, RigError> {
    let mut dependency_map: HashMap<&ComponentHandle, HashSet<ComponentHandle>> = HashMap::new();
    let mut component_evaluate_input_params: HashMap<&ComponentHandle, EvaluateInputParams> =
        HashMap::new();

    for component_state in state.component_states.values() {
        // Get the input of the component, which is either the input_override or the input or None.
        let input = component_state.input();

        // Find all the JSON path strings in the input of the component.
        let json_path_strings = match input {
            Some(input) => find_json_path_strings::find_json_path_strings(input),
            None => Vec::new(),
        };

        // Extract the component's dependencies from the JSON path strings.
        let component_dependencies = json_path_strings.extract_dependencies()?;

        // The component can execute if all of it's dependencies have an execution_output.
        let can_execute = component_dependencies.iter().all(|d| {
            state
                .get_component_state(d)
                .expect("component should exist in component states")
                .output()
                .is_some()
        });

        if can_execute {
            // The component can execute, so add it to the list of inputs we need to evaluate.
            component_evaluate_input_params.insert(
                component_state.handle,
                EvaluateInputParams {
                    input,
                    json_path_strings,
                },
            );
        }

        dependency_map.insert(component_state.handle, component_dependencies);
    }

    let dependency_map_refs =
        map_dependencies_to_rig_handles::map_dependencies_to_rig_handles(dependency_map)?;

    let sorted_and_grouped = sort_and_group(&dependency_map_refs)?;
    let execution_order = sorted_and_grouped.sorted;
    let component_groups = sorted_and_grouped.grouped;

    let mut execution_inputs: HashMap<&ComponentHandle, ComponentInput> = HashMap::new();

    // We have to evaluate the inputs in topological order because they may refer to the
    // evaluated inputs of their dependencies.
    if !component_evaluate_input_params.is_empty() {
        // Serialize the rig state to a JSON value.
        let mut serialized_rig_state = serde_json::to_value(&state.session.rig)
            .map_err(|error| RigError::RigParseFailed { error })?;

        // Add the additional rig context to the serialized rig state.
        serialized_rig_state[RIG_CONTEXT_KEY] =
            state.session.options.rig_additional_context.clone();

        // For each component handle, in execution order.
        for &component_handle in execution_order.iter() {
            // Get the current component state.
            let component_state = state.get_component_state(component_handle)?;

            // Get the component output, which is either the output_override or the
            // execution_output or None.
            let output = component_state.output();

            // If the component has output, then set it in the serialized rig state.
            if let Some(output) = output {
                serialized_rig_state[RIGGING_KEY][&component_handle.0][OUTPUT_KEY] = output.clone();
            }

            // If the component can execute...
            if let Some(evaluate_input_params) =
                component_evaluate_input_params.get(component_handle)
            {
                // Evaluate the execution input on the latest serialized rig state.
                let execution_input = evaluate_input::evaluate_input(
                    component_handle,
                    &serialized_rig_state,
                    evaluate_input_params.input,
                    &evaluate_input_params.json_path_strings,
                )?;

                validate_component_io_from_session(
                    state.session,
                    component_state,
                    ValidationData::Input(&execution_input.value),
                )?;

                // Set the execution input in the serialized rig state (in case
                // later components reference this component's input).
                serialized_rig_state[RIGGING_KEY][&component_handle.0][INPUT_KEY] =
                    execution_input.value.clone();

                // Insert the execution input into the execution inputs map.
                // We can't set it on the component state immediately because it is immutable.
                execution_inputs.insert(component_handle, execution_input);
            }
        }
    }

    // Make the state mutable and update it.
    let mut state = state;

    // Update the execution order, which may have changed if inputs were overridden.
    state.valid_execution_order = execution_order;
    state.component_groups = component_groups;

    // Update the execution input of every component.
    for key in state.session.rig.rigging.components.keys() {
        let component_state = state.get_component_state_mut(key)?;
        component_state.execution_input = execution_inputs.remove(key).map(Arc::new);
        component_state.dependencies.clone_from(
            dependency_map_refs
                .get(key)
                .expect("component should exist in dependency map"),
        );
    }

    Ok(state)
}

struct EvaluateInputParams<'rig> {
    input: Option<&'rig serde_json::Value>,
    json_path_strings: Vec<FoundJsonPathString<'rig>>,
}
