use slipway_lib::errors::RigError;
use slipway_wasmtime::WasmExecutionError;
use thiserror::Error;

use crate::canvas::CanvasError;

#[derive(Error, Debug)]
pub enum SlipwayDebugError {
    #[error("Slipway error: {0}")]
    SlipwayError(#[from] RigError),

    #[error("{0}")]
    UserError(String),

    #[error("{0}")]
    ComponentError(String),

    #[error("Parsing JSON from text editor failed.\n{0}")]
    ParseFailed(#[from] serde_json::Error),

    #[error("Failed to execute WASM.\n{0}")]
    WasmExecutionFailed(#[from] WasmExecutionError),

    #[error("{0}")]
    CanvasError(CanvasError),
}
