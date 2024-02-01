use std::collections::HashSet;

use crate::{errors::SlipwayError, rigging::parse::App};

mod determine_valid_instructions;
mod extract_dependencies_from_json_path_strings;
mod find_json_path_strings;
mod get_rigging_component_names_from_json_path_strings;
mod parse_json_path_strings;
mod topological_sort;

use find_json_path_strings::find_json_path_strings;
use serde::{Deserialize, Serialize};
use topological_sort::topological_sort;

use extract_dependencies_from_json_path_strings::ExtractDependencies;

use super::parse::ComponentHandle;

pub(crate) fn initialize(app: App) -> Result<ExecutionState, SlipwayError> {
    let mut components_with_dependencies = Vec::new();
    for (key, rigging) in app.rigging.components.iter() {
        let input = &rigging.input;

        // Find all the JSON path strings in the input of the component.
        let json_path_strings = match input {
            Some(input) => find_json_path_strings(input),
            None => Vec::new(),
        };

        // Extract the component's dependencies from the JSON path strings.
        let dependencies = json_path_strings.extract_dependencies()?;

        components_with_dependencies.push(ComponentAndDependencies {
            component_handle: key.clone(),
            input_handles: dependencies,
        });
    }

    // Get the execution order.
    let order = topological_sort(&components_with_dependencies)?;

    // Evaluate valid instructions.
    todo!();

    Ok(ExecutionState {
        app,
        execution_order: order.into_iter().cloned().collect(),
        valid_instructions: Vec::new(),
        last_instruction: None,
    })
}

pub(crate) fn step(
    state: &ExecutionState,
    instruction: &Instruction,
) -> Result<ExecutionState, SlipwayError> {
    // Note: When we expose an API outside of the crate we should not accept an App back from the
    // caller, as modifying the entire app (including permissions) could be a security risk depending
    // on whether we detect this and inform the user.
    // Instead we should return a reference to the execution session, which the caller can use to
    // step through the execution.
    todo!();
}

struct ComponentAndDependencies {
    component_handle: ComponentHandle,
    input_handles: HashSet<ComponentHandle>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExecutionState {
    pub app: App,
    pub execution_order: Vec<ComponentHandle>,
    pub valid_instructions: Vec<Instruction>,
    pub last_instruction: Option<Instruction>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "operation")]
pub(crate) enum Instruction {
    PrepareInput {
        handle: ComponentHandle,
    },
    ReplaceInput {
        handle: ComponentHandle,
        input: serde_json::Value,
    },
    Execute {
        handle: ComponentHandle,
    },
    AssessOutput {
        handle: ComponentHandle,
    },
    ReplaceOutput {
        handle: ComponentHandle,
        output: serde_json::Value,
    },
}
