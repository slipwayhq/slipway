use std::collections::{HashMap, HashSet};

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

pub(crate) fn create_session(app: App) -> AppSession {
    AppSession { app }
}

// Convert the dependency map to use references to the component handles in the AppSession.
fn map_dependencies_to_app_handles(
    dependency_map: HashMap<&ComponentHandle, HashSet<ComponentHandle>>,
) -> Result<HashMap<&ComponentHandle, HashSet<&ComponentHandle>>, SlipwayError> {
    let mut result: HashMap<&ComponentHandle, HashSet<&ComponentHandle>> = HashMap::new();
    for (&k, v) in dependency_map.iter() {
        let mut refs = HashSet::with_capacity(v.len());
        for d in v {
            let lookup_result = dependency_map.get_key_value(d);
            let kr = match lookup_result {
                Some((kr, _)) => kr,
                None => {
                    return Err(SlipwayError::ValidationFailed(format!(
                        "dependency {:?} not found in rigging component keys",
                        d
                    )))
                }
            };
            refs.insert(*kr);
        }

        result.insert(k, refs);
    }

    Ok(result)
}

pub fn initialize<'app>(
    session: &'app AppSession,
) -> Result<AppExecutionState<'app>, SlipwayError> {
    let mut dependency_map: HashMap<&'app ComponentHandle, HashSet<ComponentHandle>> =
        HashMap::new();
    for (key, rigging) in session.app.rigging.components.iter() {
        let input = &rigging.input;

        // Find all the JSON path strings in the input of the component.
        let json_path_strings = match input {
            Some(input) => find_json_path_strings(input),
            None => Vec::new(),
        };

        // Extract the component's dependencies from the JSON path strings.
        let component_dependencies = json_path_strings.extract_dependencies()?;

        dependency_map.insert(key, component_dependencies);
    }

    let mut dependency_map_refs = map_dependencies_to_app_handles(dependency_map)?;

    // Get the execution order.
    let execution_order = topological_sort(&dependency_map_refs)?;

    let dependency_map_refs = &mut dependency_map_refs;
    let component_states = execution_order
        .into_iter()
        .map(|handle| {
            let dependencies: HashSet<&ComponentHandle> = dependency_map_refs
                .remove(handle)
                .expect("component handle should exist in dependencies");

            let can_execute = dependencies.is_empty();

            ComponentState {
                handle,
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
        session,
        component_states,
    })
}

pub fn step<'app>(
    state: &'app AppExecutionState,
    instruction: Instruction,
) -> Result<AppExecutionState<'app>, SlipwayError> {
    todo!();
}

trait ExecuteWasm {
    fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, SlipwayError>;
}

pub struct AppSession {
    app: App,
}

pub struct AppExecutionState<'app> {
    session: &'app AppSession,
    component_states: Vec<ComponentState<'app>>,
}

impl<'app> AppExecutionState<'app> {
    pub fn component_states(&self) -> &[ComponentState] {
        &self.component_states
    }
}

pub struct ComponentState<'app> {
    pub handle: &'app ComponentHandle,
    pub input_override: Option<ComponentInput>,
    pub input_evaluated: Option<ComponentInput>,
    pub output_component: Option<ComponentOutput>,
    pub output_override: Option<ComponentOutput>,
    pub dependencies: HashSet<&'app ComponentHandle>,
    pub can_execute: bool,
}

pub struct ComponentInput {
    value: serde_json::Value,
    hash: Hash,
}

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
