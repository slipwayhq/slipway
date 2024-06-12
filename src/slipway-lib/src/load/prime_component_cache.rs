use itertools::Itertools;

use crate::{errors::ComponentLoadError, load::ComponentsLoader, App, Component};

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

        let definition = Component::<jtd::Schema> {
            publisher: parsed_definition.publisher,
            name: parsed_definition.name,
            version: parsed_definition.version,
            description: parsed_definition.description,
            input: jtd::Schema::from_serde_schema(parsed_definition.input)?,
            output: jtd::Schema::from_serde_schema(parsed_definition.output)?,
        };

        component_cache.add(
            loaded_component.reference,
            definition,
            loaded_component.wasm_bytes,
        );
    }

    Ok(component_cache)
}
