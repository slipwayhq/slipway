use std::sync::Arc;

use slipway_engine::{CallChain, ComponentExecutionContext, Permission};
use tracing::warn;

use crate::ComponentError;

pub fn ensure_can_query_font(
    query: &str,
    execution_context: &ComponentExecutionContext,
) -> Result<(), ComponentError> {
    ensure_can_query_font_inner(query, Arc::clone(&execution_context.call_chain))
}

fn ensure_can_query_font_inner(
    query: &str,
    call_chain: Arc<CallChain<'_>>,
) -> Result<(), ComponentError> {
    let is_allowed = slipway_engine::ensure_permissions(Arc::clone(&call_chain), |permissions| {
        fn matches(query: &str, permission: &Permission) -> bool {
            match permission {
                Permission::All => true,
                Permission::FontQuery(permission) => permission.matches(query),
                _ => false,
            }
        }

        for permission in permissions.deny {
            if matches(query, permission) {
                warn!(
                    "Component {} denied access to font query {} with deny permission {:?}",
                    call_chain.component_handle_trail(),
                    query,
                    permission
                );
                return false;
            }
        }

        for permission in permissions.allow {
            if matches(query, permission) {
                return true;
            }
        }

        warn!(
            "Component {} denied access to font query {}. No appropriate allow permission found.",
            call_chain.component_handle_trail(),
            query
        );

        false
    });

    if !is_allowed {
        return Err(ComponentError {
            message: format!(
                "Component {} does not have permission to perform font query {}",
                call_chain.component_handle_trail(),
                query
            ),
            inner: vec![],
        });
    }

    Ok(())
}
