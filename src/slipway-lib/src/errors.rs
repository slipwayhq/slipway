use thiserror::Error;

pub(crate) const INVALID_COMPONENT_REFERENCE: &str = "Invalid component reference";

#[derive(Error, Debug)]
pub enum SlipwayError {
    #[error("Rigging resolve failed")]
    RiggingResolveFailed(String),

    #[error("Rigging parse failed")]
    RiggingParseFailed(#[from] serde_json::Error),

    #[error("Rigging validation failed: {0}")]
    RiggingValidationFailed(String),

    // If this error is generated during Serde deserialization it will be converted
    // into a `serde_json::Error` and wrapped in a RiggingParseFailed exception.
    #[error("{INVALID_COMPONENT_REFERENCE}: {0}")]
    InvalidComponentReference(String),
}
