use async_trait::async_trait;

use crate::errors::SlipwayError;

use super::{
    parse::types::ComponentReference,
    recursive_resolve::{
        load::{ComponentRigging, LoadComponentRigging, LoadError},
        recursively_resolve_components, Context,
    },
};

pub fn build_resolved_component(root_component: String) -> Result<(), SlipwayError> {
    let component_reference_resolver = Box::new(LoadComponentRiggingImpl {});
    build_resolved_component_with_resolver(root_component, component_reference_resolver)
}

pub(crate) fn build_resolved_component_with_resolver(
    root_component: String,
    component_reference_resolver: Box<dyn LoadComponentRigging>,
) -> Result<(), SlipwayError> {
    let _resolved_components =
        recursively_resolve_components(root_component, component_reference_resolver);
    todo!("Build full hierarchy");
}

struct LoadComponentRiggingImpl {}

#[async_trait]
impl LoadComponentRigging for LoadComponentRiggingImpl {
    async fn load_component_rigging<'a, 'b>(
        &self,
        _reference: ComponentReference,
        _context: &'a Context<'a>,
    ) -> Result<ComponentRigging<'b>, LoadError<'b>>
    where
        'a: 'b,
    {
        todo!();
    }
}
