use thiserror::Error;

pub(crate) const INVALID_COMPONENT_REFERENCE: &str = "invalid component reference";
pub(crate) const INVALID_RESOLVED_COMPONENT_REFERENCE: &str =
    "invalid resolved component reference";

#[derive(Error, Debug)]
pub enum SlipwayError {
    #[error("rigging resolve failed")]
    RiggingResolveFailed(String),

    #[error("rigging parse failed")]
    RiggingParseFailed(#[from] serde_json::Error),

    #[error("rigging validation failed: {0}")]
    RiggingValidationFailed(String),

    // If this error is generated during Serde deserialization it will be converted
    // into a `serde_json::Error` and wrapped in a RiggingParseFailed exception.
    #[error("{INVALID_COMPONENT_REFERENCE}: {0}")]
    InvalidComponentReference(String),

    // If this error is generated during Serde deserialization it will be converted
    // into a `serde_json::Error` and wrapped in a RiggingParseFailed exception.
    #[error("{INVALID_RESOLVED_COMPONENT_REFERENCE}: {0}")]
    InvalidResolvedComponentReference(String),
}
