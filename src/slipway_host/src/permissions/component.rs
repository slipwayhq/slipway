use std::{path::Path, sync::Arc};

use crate::ComponentError;
use slipway_engine::{
    CallChain, ComponentExecutionContext, ComponentHandle, Permission, SlipwayReference,
};
use url::Url;

pub fn ensure_can_use_component_handle(
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

    let call_chain = Arc::clone(&execution_context.call_chain);

    ensure_can_use_component_reference(component_reference, call_chain)
}

pub fn ensure_can_use_component_reference(
    component_reference: &SlipwayReference,
    call_chain: Arc<CallChain<'_>>,
) -> Result<(), ComponentError> {
    let is_allowed = match component_reference {
        SlipwayReference::Registry {
            publisher: _,
            name: _,
            version: _,
        } => {
            fn matches(reference: &str, permission: &Permission) -> bool {
                match permission {
                    Permission::All => true,
                    Permission::RegistryComponent(permission) => permission.matches(reference),
                    _ => false,
                }
            }

            let reference = component_reference.to_string();
            slipway_engine::ensure_permissions(call_chain.clone(), |permissions| {
                for permission in permissions.deny {
                    if matches(&reference, permission) {
                        return false;
                    }
                }
                for permission in permissions.allow {
                    if matches(&reference, permission) {
                        return true;
                    }
                }
                false
            })
        }
        SlipwayReference::Url { url } => {
            fn matches(url: &Url, permission: &Permission) -> bool {
                match permission {
                    Permission::All => true,
                    Permission::HttpComponent(permission) => permission.matches(url),
                    _ => false,
                }
            }

            slipway_engine::ensure_permissions(call_chain.clone(), |permissions| {
                for permission in permissions.deny {
                    if matches(url, permission) {
                        return false;
                    }
                }
                for permission in permissions.allow {
                    if matches(url, permission) {
                        return true;
                    }
                }
                false
            })
        }
        SlipwayReference::Local { path } => {
            fn matches(path: &Path, permission: &Permission) -> bool {
                match permission {
                    Permission::All => true,
                    Permission::FileComponent(permission) => permission.matches(path),
                    _ => false,
                }
            }

            slipway_engine::ensure_permissions(call_chain.clone(), |permissions| {
                for permission in permissions.deny {
                    if matches(path, permission) {
                        return false;
                    }
                }
                for permission in permissions.allow {
                    if matches(path, permission) {
                        return true;
                    }
                }
                false
            })
        }
        SlipwayReference::Special(_) => true,
    };

    if !is_allowed {
        return Err(ComponentError {
            message: format!(
                "Component {} does not have permission to access component {}",
                call_chain.component_handle_trail(),
                component_reference
            ),
            inner: vec![],
        });
    }

    Ok(())
}
