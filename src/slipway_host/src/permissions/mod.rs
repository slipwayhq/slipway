mod component;
mod font;
mod http_fetch;

pub use component::ensure_can_use_component_handle;
pub use component::ensure_can_use_component_reference;
pub use font::ensure_can_query_font;
pub use http_fetch::ensure_can_fetch_url;
use slipway_engine::CallChain;
use slipway_engine::Permission;
use tracing::debug;
use tracing::warn;

use crate::ComponentError;

fn warn_deny_permission_triggered(permission: &Permission) {
    warn!("Deny permission triggered: {:?}", permission);
}

fn create_permission_error(message: String, call_chain: &CallChain<'_>) -> ComponentError {
    let permissions = format!("Permission Chain: {:?}", call_chain.permission_trail());
    warn!(message);
    debug!(permissions);
    ComponentError {
        message,
        inner: vec![permissions],
    }
}
