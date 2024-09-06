use std::sync::Arc;

use itertools::Itertools;
use jsonschema::JSONSchema;

use crate::{errors::ComponentLoadError, load::ComponentsLoader, App, Component, Schema};

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

fn parse_schema(schema: serde_json::Value) -> Result<Schema, ComponentLoadError> {
    if let Some(serde_json::Value::String(schema_uri)) = schema.get("$schema") {
        if schema_uri.contains("://json-schema.org/") {
            // If the schema contains a $schema property, and the domain is json-schema.org, it is a JSON Schema.
            let compiled_schema = JSONSchema::compile(&schema)
                .map_err(|e| ComponentLoadError::JsonSchemaParseFailed(e.into()))?;

            return Ok(Schema::JsonSchema(compiled_schema));
        }
    }

    // Otherwise it is JsonTypeDef.
    let jtd_serde_schema: jtd::SerdeSchema = serde_json::from_value(schema)
        .map_err(|e| ComponentLoadError::DefinitionParseFailed(Arc::new(e)))?;

    let jtd_schema = jtd::Schema::from_serde_schema(jtd_serde_schema)?;

    Ok(Schema::JsonTypeDef(jtd_schema))
}
