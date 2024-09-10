use itertools::Itertools;

use crate::{
    errors::ComponentLoadError, load::ComponentsLoader, parse_schema, App, Component, Schema,
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

        let parsed_definition = crate::parse::parse_component(&loaded_component.definition)?;

        let input = parse_schema(parsed_definition.input)?;
        let output = parse_schema(parsed_definition.output)?;

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
            loaded_component.wasm_bytes,
        );
    }

    Ok(component_cache)
}
