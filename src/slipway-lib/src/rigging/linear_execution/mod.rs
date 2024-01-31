use std::collections::HashSet;

use crate::{errors::SlipwayError, rigging::parse::App};

mod dependency_graph;
mod extract_dependencies_from_json_path_strings;
mod find_json_path_strings;
mod get_rigging_component_names_from_json_path_strings;
mod parse_json_path_strings;

use find_json_path_strings::find_json_path_strings;

use self::extract_dependencies_from_json_path_strings::ExtractDependencies;

use super::parse::ComponentHandle;

pub fn initialize(app: &App) -> Result<(), SlipwayError> {
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

    Ok(())
}

struct ComponentAndDependencies {
    component_handle: ComponentHandle,
    input_handles: HashSet<ComponentHandle>,
}
