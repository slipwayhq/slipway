use async_trait::async_trait;

use crate::errors::SlipwayError;

use super::{
    parse::types::ComponentReference,
    resolve::{
        resolve_components, BuildContext, ComponentReferenceResolveError,
        ComponentReferenceResolver, ResolvedReferenceContent,
    },
};

pub fn build_resolved_component(root_component: String) -> Result<(), SlipwayError> {
    let component_reference_resolver = Box::new(ComponentReferenceResolverImpl {});
    build_resolved_component_with_resolver(root_component, component_reference_resolver)
}

pub(crate) fn build_resolved_component_with_resolver(
    root_component: String,
    component_reference_resolver: Box<dyn ComponentReferenceResolver>,
) -> Result<(), SlipwayError> {
    let _resolved_components = resolve_components(root_component, component_reference_resolver);
    // TODO: Build full hierarchy.
    Ok(())
}

struct ComponentReferenceResolverImpl {}

#[async_trait]
impl ComponentReferenceResolver for ComponentReferenceResolverImpl {
    async fn resolve(
        &self,
        _reference: ComponentReference,
        _context: BuildContext,
    ) -> Result<ResolvedReferenceContent, ComponentReferenceResolveError> {
        todo!();
    }
}
