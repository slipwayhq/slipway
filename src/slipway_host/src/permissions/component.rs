use std::sync::Arc;

use slipway_engine::{
    CallChain, ComponentExecutionContext, ComponentHandle, Permission, SlipwayReference,
    StringPermission, UrlPermission,
};
use url::Url;
use crate::ComponentError;

pub fn ensure_can_use_component(
    handle: &ComponentHandle,
    execution_context: &ComponentExecutionContext,
) -> Result<(), ComponentError> {
    let component_reference = execution_context
        .callout_context
        .get_component_reference_for_handle(handle)
        .map_err(|e| ComponentError {
            message: format!(
                "Failed to component reference for \"{}\"",
                execution_context
                    .call_chain
                    .component_handle_trail_for(handle)
            ),
            inner: vec![format!("{e}")],
        })?;

    if let SlipwayReference::Local { path } = &component_reference {
        if !slipway_engine::ensure_permissions(
            Arc::clone(&execution_context.call_chain),
            |permissions| {
                for permission in permissions.allow {
                    match permission {
                        Permission::FileComponent(StringPermission::Any) | Permission::All => {
                            return true
                        }
                        _ => {}
                    }
                }
                false
            },
        ) {
            return Err(ComponentError {
                message: format!(
                    "Component {} does not have permission to access local component {}",
                    execution_context.call_chain.component_handle_trail(),
                    handle
                ),
                inner: vec![format!("Local component path: {}", path.to_string_lossy())],
            });
        }
    }

    Ok(())
}
