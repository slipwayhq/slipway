use serde::{Deserialize, Serialize};

pub mod bin;
pub mod fetch;
pub mod fonts;
pub mod log;
mod permissions;
pub mod run;

pub const SLIPWAY_COMPONENT_WASM_FILE_NAME: &str = "slipway_component.wasm";

// We can't use the Wasmtime/WIT generated ComponentError here, as this crate is host independent,
// so use our own struct.
#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentError {
    pub message: String,
    pub inner: Vec<String>,
}

impl ComponentError {
    pub fn for_error(message: String, error: Option<String>) -> ComponentError {
        ComponentError {
            message,
            inner: match error {
                None => vec![],
                Some(e) => vec![format!("{}", e)],
            },
        }
    }
}
