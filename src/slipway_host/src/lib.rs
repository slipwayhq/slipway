pub mod fetch;
pub mod fonts;
pub mod log;
pub mod run;
mod permissions;

pub const SLIPWAY_COMPONENT_WASM_FILE_NAME: &str = "slipway_component.wasm";

// We can't use the Wasmtime/WIT generated ComponentError here, as this crate is host independent,
// so use our own struct.
pub struct ComponentError {
    pub message: String,
    pub inner: Vec<String>,
}
