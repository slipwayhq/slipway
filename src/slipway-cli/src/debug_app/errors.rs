use slipway_lib::errors::SlipwayError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SlipwayDebugError {
    #[error("slipway error: {0}")]
    SlipwayError(#[from] SlipwayError),

    #[error("{0}")]
    UserError(String),

    #[error("parsing JSON from text editor failed")]
    ParseFailed(#[from] serde_json::Error),
}
