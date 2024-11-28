use std::collections::{HashMap, HashSet};

use itertools::Itertools;

use crate::{
    errors::{ComponentLoadError, ComponentLoadErrorInner},
    load::ComponentsLoader,
    parse::parse_schema,
    parse_component, Component, ComponentCallout, ComponentHandle, Rig, Schema, SlipwayReference,
};

use super::ComponentCache;

pub(super) fn prime_component_cache(
    rig: &Rig,
    components_loader: &impl ComponentsLoader,
) -> Result<ComponentCache, ComponentLoadError> {
    let mut component_cache = ComponentCache::empty();
    let mut pending_component_references = get_rig_distinct_references(rig);
    let mut loaded_component_references: HashSet<SlipwayReference> = HashSet::new();

    while !pending_component_references.is_empty() {
        let next = pending_component_references.drain().collect_vec();
        let loaded_components = components_loader.load_components(&next);
        loaded_component_references.extend(next);

        for maybe_loaded_component in loaded_components {
            let loaded_component = maybe_loaded_component?;

            let parsed_definition = handle_component_load_error(
                &loaded_component.reference,
                parse_component(&loaded_component.definition),
            )?;

            let input = handle_component_load_error(
                &loaded_component.reference,
                parse_schema(
                    "input",
                    parsed_definition.input,
                    loaded_component.files.clone(),
                ),
            )?;

            let output = handle_component_load_error(
                &loaded_component.reference,
                parse_schema(
                    "output",
                    parsed_definition.output,
                    loaded_component.files.clone(),
                ),
            )?;

            let definition = Component::<Schema> {
                publisher: parsed_definition.publisher,
                name: parsed_definition.name,
                version: parsed_definition.version,
                description: parsed_definition.description,
                input,
                output,
                constants: parsed_definition.constants,
                rigging: parsed_definition.rigging,
                callouts: parsed_definition.callouts,
            };

            let new_references = {
                let mut all_references = get_component_distinct_references(&definition);
                all_references.retain(|r| !loaded_component_references.contains(r));
                all_references
            };

            pending_component_references.extend(new_references);

            component_cache.add(
                &loaded_component.reference,
                definition,
                loaded_component.files,
            );
        }
    }

    Ok(component_cache)
}

fn handle_component_load_error<T>(
    reference: &SlipwayReference,
    result: Result<T, ComponentLoadErrorInner>,
) -> Result<T, ComponentLoadError> {
    result.map_err(|e| ComponentLoadError::new(reference, e))
}

fn get_rig_distinct_references(rig: &Rig) -> HashSet<SlipwayReference> {
    rig.rigging
        .components
        .values()
        .flat_map(|v| std::iter::once(&v.component).chain(get_callouts_references(&v.callouts)))
        .unique()
        .cloned()
        .collect()
}

fn get_component_distinct_references<T>(component: &Component<T>) -> HashSet<SlipwayReference> {
    component
        .rigging
        .as_ref()
        .map(|rigging| {
            rigging.components.values().flat_map(|v| {
                std::iter::once(&v.component).chain(get_callouts_references(&v.callouts))
            })
        })
        .into_iter()
        .flatten()
        .chain(get_callouts_references(&component.callouts))
        .unique()
        .cloned()
        .collect()
}

fn get_callouts_references<'a>(
    callouts: &'a Option<HashMap<ComponentHandle, ComponentCallout>>,
) -> Box<dyn Iterator<Item = &'a SlipwayReference> + 'a> {
    match callouts {
        Some(callouts) => Box::new(callouts.values().flat_map(|v| {
            std::iter::once(&v.component).chain(get_callouts_references(&v.callouts))
        })),
        None => Box::new(std::iter::empty()),
    }
}
