use itertools::Itertools;

use crate::{
    errors::{ComponentLoadError, ComponentLoadErrorInner},
    load::ComponentsLoader,
    parse::parse_schema,
    parse_component, App, Component, Schema, SlipwayReference,
};

use super::ComponentCache;

pub(super) fn prime_component_cache(
    app: &App,
    components_loader: &impl ComponentsLoader,
) -> Result<ComponentCache, ComponentLoadError> {
    let distinct_component_references: Vec<_> = app
        .rigging
        .components
        .values()
        .map(|v| &v.component)
        .unique()
        .collect();

    let loaded_components = components_loader.load_components(&distinct_component_references);

    let mut component_cache = ComponentCache::empty();

    for maybe_loaded_component in loaded_components {
        let loaded_component = maybe_loaded_component?;

        let parsed_definition = handle_component_load_error(
            loaded_component.reference,
            parse_component(&loaded_component.definition),
        )?;

        let input = handle_component_load_error(
            loaded_component.reference,
            parse_schema(
                "input",
                parsed_definition.input,
                loaded_component.json.clone(),
            ),
        )?;

        let output = handle_component_load_error(
            loaded_component.reference,
            parse_schema(
                "output",
                parsed_definition.output,
                loaded_component.json.clone(),
            ),
        )?;

        let definition = Component::<Schema> {
            publisher: parsed_definition.publisher,
            name: parsed_definition.name,
            version: parsed_definition.version,
            description: parsed_definition.description,
            input,
            output,
        };

        component_cache.add(
            loaded_component.reference,
            definition,
            loaded_component.wasm,
            loaded_component.json,
        );
    }

    Ok(component_cache)
}

fn handle_component_load_error<T>(
    reference: &SlipwayReference,
    result: Result<T, ComponentLoadErrorInner>,
) -> Result<T, ComponentLoadError> {
    result.map_err(|e| ComponentLoadError::new(reference, e))
}
