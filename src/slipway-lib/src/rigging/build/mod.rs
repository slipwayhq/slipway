use crate::errors::SlipwayError;

use super::resolve::{resolve_components, ComponentReferenceResolver};

pub(crate) fn build_resolved_component(
    root_component: String,
    component_reference_resolver: Box<dyn ComponentReferenceResolver>,
) -> Result<(), SlipwayError> {
    let _resolved_components = resolve_components(root_component, component_reference_resolver);
    // TODO: Build full hierarchy.
    Ok(())
}
