use std::str::FromStr;

use slipway_engine::{ComponentExecutionContext, ComponentHandle};

pub mod fonts;
pub mod http;
pub mod load;
pub mod run;

pub const SLIPWAY_COMPONENT_WASM_FILE_NAME: &str = "slipway_component.wasm";

// We can't use the Wasmtime/WIT generated ComponentError here, as this crate is host independent,
// so use our own struct.
pub struct ComponentError {
    pub message: String,
}

fn parse_handle(
    execution_context: &ComponentExecutionContext,
    handle: &str,
) -> Result<ComponentHandle, ComponentError> {
    ComponentHandle::from_str(handle).map_err(|e| ComponentError {
        message: format!(
            "Failed to parse component handle \"{}\" from \"{}\":\n{}",
            handle,
            execution_context.call_chain.component_handle_trail(),
            e
        ),
    })
}
