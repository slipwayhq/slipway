use slipway_engine::{RunComponentError, RunError, errors::RigError};
use thiserror::Error;

use crate::{canvas::CanvasError, host_error::HostError};

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

    #[error("Failed to execute component.\n{0}")]
    ComponentExecutionFailed(#[from] RunComponentError),

    #[error("{0}")]
    CanvasError(CanvasError),

    #[error("{0}")]
    RunError(#[from] RunError<HostError>),
}
