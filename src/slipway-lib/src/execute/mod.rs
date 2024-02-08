use std::collections::HashMap;

use crate::{
    errors::SlipwayError,
    parse::types::{primitives::ComponentHandle, App},
};

mod extract_dependencies_from_json_path_strings;
mod find_json_path_strings;
mod get_rigging_component_names_from_json_path_strings;
mod hash_json_value;
mod parse_json_path_strings;
mod primitives;
mod topological_sort;

use find_json_path_strings::find_json_path_strings;
use serde::{Deserialize, Serialize};
use topological_sort::topological_sort;

use extract_dependencies_from_json_path_strings::ExtractDependencies;
use primitives::Hash;

pub(crate) fn initialize(app: App) -> Result<AppExecutionState, SlipwayError> {
    let mut dependencies = HashMap::new();
    for (key, rigging) in app.rigging.components.iter() {
        let input = &rigging.input;

        // Find all the JSON path strings in the input of the component.
        let json_path_strings = match input {
            Some(input) => find_json_path_strings(input),
            None => Vec::new(),
        };

        // Extract the component's dependencies from the JSON path strings.
        let component_dependencies = json_path_strings.extract_dependencies()?;

        dependencies.insert(key.clone(), component_dependencies);
    }

    // Ensure all dependencies are also in the map as keys.
    for dependency in dependencies.values().flatten() {
        if !dependencies.contains_key(dependency) {
            return Err(SlipwayError::ValidationFailed(format!(
                "dependency {:?} not found in component keys",
                dependency
            )));
        }
    }

    // Get the execution order.
    let execution_order = topological_sort(&dependencies)?;

    let component_states = execution_order
        .into_iter()
        .map(|handle| {
            let dependencies: Vec<ComponentHandle> = dependencies
                .get(handle)
                .expect("component handle should exist in dependencies")
                .iter()
                .cloned()
                .collect();

            let can_execute = dependencies.is_empty();

            ComponentState {
                handle: handle.clone(),
                input_override: None,
                input_evaluated: None,
                output_component: None,
                output_override: None,
                dependencies,
                can_execute,
            }
        })
        .collect();

    // Evaluate valid instructions.
    Ok(AppExecutionState {
        app,
        component_states,
    })
}

pub(crate) fn step(
    state: &AppExecutionState,
    instruction: &Instruction,
) -> Result<AppExecutionState, SlipwayError> {
    todo!();
}

trait ExecuteWasm {
    fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, SlipwayError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppExecutionState {
    app: App,
    component_states: Vec<ComponentState>,
}

impl AppExecutionState {
    pub fn component_states(&self) -> &[ComponentState] {
        &self.component_states
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentState {
    pub handle: ComponentHandle,
    pub input_override: Option<ComponentInput>,
    pub input_evaluated: Option<ComponentInput>,
    pub output_component: Option<ComponentOutput>,
    pub output_override: Option<ComponentOutput>,
    pub dependencies: Vec<ComponentHandle>,
    pub can_execute: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentInput {
    value: serde_json::Value,
    hash: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentOutput {
    value: serde_json::Value,
    input_hash_used: Hash,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "operation")]
#[serde(rename_all = "snake_case")]
enum Instruction {
    SetInput {
        handle: ComponentHandle,
        value: serde_json::Value,
    },
    EvaluateInput {
        handle: ComponentHandle,
    },
    SetOutput {
        handle: ComponentHandle,
        value: serde_json::Value,
    },
    ExecuteComponent {
        handle: ComponentHandle,
    },
}
