use thiserror::Error;

pub(crate) const INVALID_SLIPWAY_ID: &str = "invalid slipway id";
pub(crate) const INVALID_SLIPWAY_REFERENCE: &str = "invalid slipway reference";

#[derive(Error, Debug)]
pub enum SlipwayError {
    #[error("resolve failed")]
    ResolveFailed(String),

    #[error("parse failed")]
    ParseFailed(#[from] serde_json::Error),

    #[error("validation failed: {0}")]
    ValidationFailed(String),

    // If this error is generated during Serde deserialization it will be converted
    // into a `serde_json::Error` and wrapped in a ParseFailed exception.
    #[error("{INVALID_SLIPWAY_REFERENCE}: {0}")]
    InvalidSlipwayReference(String),

    // If this error is generated during Serde deserialization it will be converted
    // into a `serde_json::Error` and wrapped in a ParseFailed exception.
    #[error("{INVALID_SLIPWAY_ID}: {0}")]
    InvalidSlipwayId(String),
}
