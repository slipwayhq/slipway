use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use crate::{errors::SlipwayError, types::App};

mod extract_dependencies_from_json_path_strings;
mod find_json_path_strings;
mod get_rigging_component_names_from_json_path_strings;
mod get_valid_instructions;
mod parse_json_path_strings;
mod topological_sort;

use find_json_path_strings::find_json_path_strings;
use get_valid_instructions::get_valid_instructions;
use serde::{Deserialize, Serialize};
use topological_sort::topological_sort;

use extract_dependencies_from_json_path_strings::ExtractDependencies;

use super::types::primitives::ComponentHandle;

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
    let execution_order = topological_sort(&dependencies)?
        .into_iter()
        .cloned()
        .collect();

    let inputs = HashMap::new();
    let outputs = HashMap::new();

    // Evaluate valid instructions.
    let valid_instructions = get_valid_instructions(&inputs, &outputs, &dependencies);
    Ok(AppExecutionState {
        app,
        inputs,
        outputs,
        dependencies,
        execution_order,
        valid_instructions,
        last_instruction: None,
    })
}

pub(crate) fn step(
    state: &AppExecutionState,
    instruction: &ValidInstruction,
) -> Result<AppExecutionState, SlipwayError> {
    // Note: When we expose an API outside of the crate we should not accept an App back from the
    // caller, as modifying the entire app (including permissions) could be a security risk depending
    // on whether we detect this and inform the user.
    // Instead we should return a reference to the execution session, which the caller can use to
    // step through the execution.
    todo!();
}

trait ExecuteWasm {
    fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, SlipwayError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComponentInput {
    value: serde_json::Value,
    source: ComponentInputSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComponentOutput {
    value: serde_json::Value,
    source: ComponentInputSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ComponentInputSource {
    Evaluated,
    Set,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ComponentOutputSource {
    Component,
    Set,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppExecutionState {
    pub(crate) app: App,
    pub(crate) inputs: HashMap<ComponentHandle, ComponentInput>,
    pub(crate) outputs: HashMap<ComponentHandle, ComponentOutput>,
    pub(crate) dependencies: HashMap<ComponentHandle, HashSet<ComponentHandle>>,
    pub(crate) execution_order: Vec<ComponentHandle>,
    pub(crate) valid_instructions: Vec<ValidInstruction>,
    pub(crate) last_instruction: Option<ValidInstruction>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(tag = "operation")]
#[serde(rename_all = "snake_case")]
pub enum ValidInstruction {
    GetInput { handle: ComponentHandle },
    SetInput { handle: ComponentHandle },
    EvaluateInput { handle: ComponentHandle },
    GetOutput { handle: ComponentHandle },
    SetOutput { handle: ComponentHandle },
    ExecuteComponent { handle: ComponentHandle },
    GetAppOutputs,
}

impl AppExecutionState {
    pub fn get_valid_instructions(&self) -> &Vec<ValidInstruction> {
        &self.valid_instructions
    }

    pub fn get_execution_order(&self) -> &Vec<ComponentHandle> {
        &self.execution_order
    }

    pub fn get_dependencies(&self) -> &HashMap<ComponentHandle, HashSet<ComponentHandle>> {
        &self.dependencies
    }
}
