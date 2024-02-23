use crate::{errors::SlipwayError, ComponentHandle};
use std::collections::{HashMap, HashSet};

use self::{
    extract_dependencies_from_json_path_strings::ExtractDependencies,
    find_json_path_strings::FoundJsonPathString,
};

use super::{
    get_component_state, get_component_state_mut, topological_sort::topological_sort,
    AppExecutionState, ComponentInput,
};

mod evaluate_input;
mod extract_dependencies_from_json_path_strings;
mod find_json_path_strings;
mod map_dependencies_to_app_handles;
mod simple_json_path;

pub(crate) fn evaluate_component_inputs(
    state: AppExecutionState,
) -> Result<AppExecutionState, SlipwayError> {
    let mut dependency_map: HashMap<&ComponentHandle, HashSet<ComponentHandle>> = HashMap::new();
    let mut component_evaluate_input_params: HashMap<&ComponentHandle, EvaluateInputParams> =
        HashMap::new();

    for (key, component) in state.session.app.rigging.components.iter() {
        let component_state = get_component_state(&state, key)?;

        // Get the input of the component, which is either the input_override or the input or None.
        let input = component_state.input(component);

        // Find all the JSON path strings in the input of the component.
        let json_path_strings = match input {
            Some(input) => find_json_path_strings::find_json_path_strings(input),
            None => Vec::new(),
        };

        // Extract the component's dependencies from the JSON path strings.
        let component_dependencies = json_path_strings.extract_dependencies()?;

        // The component can execute if all of it's dependencies have an execution_output.
        let can_execute = component_dependencies.iter().all(|d| {
            get_component_state(&state, d)
                .expect("component should exist in component states")
                .output()
                .is_some()
        });

        if can_execute {
            // The component can execute, so add it to the list of inputs we need to evaluate.
            component_evaluate_input_params.insert(
                key,
                EvaluateInputParams {
                    input,
                    json_path_strings,
                },
            );
        }

        dependency_map.insert(key, component_dependencies);
    }

    let dependency_map_refs =
        map_dependencies_to_app_handles::map_dependencies_to_app_handles(dependency_map)?;

    let execution_order = topological_sort(&dependency_map_refs)?;
    let mut execution_inputs: HashMap<&ComponentHandle, ComponentInput> = HashMap::new();

    // We have to evaluate the inputs in topological order because they may refer to the
    // evaluated inputs of their dependencies.
    if !component_evaluate_input_params.is_empty() {
        // Serialize the app state to a JSON value.
        let mut serialized_app_state = serde_json::to_value(&state.session.app)?;

        // For each component handle, in execution order.
        for &component_handle in execution_order.iter() {
            // Get the current component state.
            let component_state = get_component_state(&state, component_handle)?;

            // Get the component output, which is either the output_override or the
            // execution_output or None.
            let output = component_state.output();

            // If the component has output, then set it in the serialized app state.
            if let Some(output) = output {
                serialized_app_state["rigging"][&component_handle.0]["output"] = output.clone();
            }

            // If the component can execute...
            if let Some(evaluate_input_params) =
                component_evaluate_input_params.get(component_handle)
            {
                // Evaluate the execution input on the latest serialized app state.
                let execution_input = evaluate_input::evaluate_input(
                    component_handle,
                    &serialized_app_state,
                    evaluate_input_params.input,
                    &evaluate_input_params.json_path_strings,
                )?;

                // Set the execution input in the serialized app state (in case
                // later components reference this component's input).
                serialized_app_state["rigging"][&component_handle.0]["input"] =
                    execution_input.value.clone();

                // Insert the execution input into the execution inputs map.
                // We can't set it on the component state immediately because it is immutable.
                execution_inputs.insert(component_handle, execution_input);
            }
        }
    }

    // Make the state mutable and update it.
    let mut state = state;
    state.execution_order = execution_order;
    for (component_handle, input) in execution_inputs {
        let component_state = get_component_state_mut(&mut state, component_handle)?;

        component_state.execution_input = Some(input);
    }

    Ok(state)
}

struct EvaluateInputParams<'app> {
    input: Option<&'app serde_json::Value>,
    json_path_strings: Vec<FoundJsonPathString<'app>>,
}
