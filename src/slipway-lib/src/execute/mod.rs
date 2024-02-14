use std::collections::{HashMap, HashSet};

use crate::{
    errors::SlipwayError,
    parse::types::{primitives::ComponentHandle, App},
};

pub(crate) mod evaluate_inputs;
mod extract_dependencies_from_json_path_strings;
mod find_json_path_strings;
mod get_rigging_component_names_from_json_path_strings;
pub(crate) mod initialize;
mod primitives;
pub(crate) mod step;
mod topological_sort;

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

fn get_component_state_mut<'app, 'local>(
    state: &'local mut AppExecutionState<'app>,
    handle: &'local ComponentHandle,
) -> Result<&'local mut crate::ComponentState<'app>, SlipwayError> {
    let component_state =
        state
            .component_states
            .get_mut(handle)
            .ok_or(SlipwayError::StepFailed(format!(
                "component {:?} does not exist in component states",
                handle
            )))?;

    Ok(component_state)
}

fn get_component_state<'app, 'local>(
    state: &'local AppExecutionState<'app>,
    handle: &'local ComponentHandle,
) -> Result<&'local crate::ComponentState<'app>, SlipwayError> {
    let component_state = state
        .component_states
        .get(handle)
        .ok_or(SlipwayError::StepFailed(format!(
            "component {:?} does not exist in component states",
            handle
        )))?;

    Ok(component_state)
}

pub struct AppSession {
    app: App,
}

pub struct AppExecutionState<'app> {
    session: &'app AppSession,
    component_states: HashMap<&'app ComponentHandle, ComponentState<'app>>,
    execution_order: Vec<&'app ComponentHandle>,
    wasm_cache: HashMap<&'app ComponentHandle, Vec<u8>>,
}

impl<'app> AppExecutionState<'app> {
    pub fn component_states(&self) -> &HashMap<&'app ComponentHandle, ComponentState> {
        &self.component_states
    }
}

pub struct ComponentState<'app> {
    pub handle: &'app ComponentHandle,
    pub dependencies: HashSet<&'app ComponentHandle>,
    pub input_override: Option<ComponentInputOverride>,
    pub output_override: Option<ComponentOutputOverride>,
    pub execution_input: Option<ComponentInput>,
    pub execution_output: Option<ComponentOutput>,
}

pub struct ComponentInput {
    value: serde_json::Value,
    hash: Hash,
}

pub struct ComponentInputOverride {
    value: serde_json::Value,
}

pub struct ComponentOutput {
    value: serde_json::Value,
    input_hash_used: Hash,
}

pub struct ComponentOutputOverride {
    value: serde_json::Value,
}
