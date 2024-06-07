use slipway_lib::errors::AppError;
use thiserror::Error;

use crate::run_component_wasm::errors::WasmExecutionError;

#[derive(Error, Debug)]
pub enum SlipwayDebugError {
    #[error("slipway error: {0}")]
    SlipwayError(#[from] AppError),

    #[error("{0}")]
    UserError(String),

    #[error("parsing JSON from text editor failed")]
    ParseFailed(#[from] serde_json::Error),

    #[error("parsing JSON from text editor failed")]
    WasmExecutionFailed(#[from] WasmExecutionError),
}
