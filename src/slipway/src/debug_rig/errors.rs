use slipway_lib::errors::RigError;
use thiserror::Error;

use crate::run_component_wasm::errors::WasmExecutionError;

#[derive(Error, Debug)]
pub enum SlipwayDebugError {
    #[error("Slipway error: {0}")]
    SlipwayError(#[from] RigError),

    #[error("{0}")]
    UserError(String),

    #[error("Parsing JSON from text editor failed.\n{0}")]
    ParseFailed(#[from] serde_json::Error),

    #[error("Failed to execute WASM.\n{0}")]
    WasmExecutionFailed(#[from] WasmExecutionError),
}
