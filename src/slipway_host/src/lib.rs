use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod bin;
pub mod fetch;
pub mod fonts;
pub mod log;
mod permissions;
pub mod render_state;
pub mod run;
pub mod tracing_writer;

pub const SLIPWAY_COMPONENT_WASM_FILE_NAME: &str = "run.wasm";

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

pub fn hash_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();
    format!("{:x}", result)
}

pub fn hash_bytes(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();
    format!("{:x}", result)
}
